use std::collections::{HashMap, VecDeque};

use crate::{
    ApplyPath, BatchProof, ChangeEnvelope, DeltaBatch, ReplicaId, RuntimeBootstrap, SnapshotBundle,
    SyncError, SyncRuntime, SyncableState,
};

#[derive(Debug, Clone)]
pub struct ChangeCtx {
    replica_id: ReplicaId,
    seq: u64,
    pending: VecDeque<DeltaBatch>,
    seen_batches: HashMap<(ReplicaId, u64), BatchProof>,
    remote_authority: Option<ReplicaId>,
    local_authority_established: bool,
}

impl ChangeCtx {
    pub fn new(replica_id: impl Into<ReplicaId>) -> Self {
        Self {
            replica_id: replica_id.into(),
            seq: 0,
            pending: VecDeque::new(),
            seen_batches: HashMap::new(),
            remote_authority: None,
            local_authority_established: false,
        }
    }

    pub fn restore(
        replica_id: impl Into<ReplicaId>,
        bootstrap: RuntimeBootstrap,
    ) -> Result<Self, SyncError> {
        let replica_id = replica_id.into();
        validate_bootstrap(&replica_id, &bootstrap)?;

        let mut pending = bootstrap.pending;
        pending.sort_by_key(|batch| batch.from_seq);

        let mut seen_batches = HashMap::new();
        for proof in bootstrap.seen_batches {
            seen_batches.insert((proof.replica_id.clone(), proof.to_seq), proof);
        }

        Ok(Self {
            replica_id,
            seq: bootstrap.seq,
            pending: pending.into(),
            seen_batches,
            remote_authority: bootstrap.remote_authority,
            local_authority_established: bootstrap.local_authority_established,
        })
    }

    pub fn replica_id(&self) -> &ReplicaId {
        &self.replica_id
    }

    pub fn current_seq(&self) -> u64 {
        self.seq
    }

    pub fn begin_batch(&mut self) -> Result<BatchTx<'_>, SyncError> {
        if let Some(remote_replica_id) = &self.remote_authority {
            return Err(SyncError::RoleConflict {
                local_replica_id: self.replica_id.clone(),
                remote_replica_id: remote_replica_id.clone(),
            });
        }

        let from_seq = self.seq;
        Ok(BatchTx {
            ctx: self,
            from_seq,
            changes: Vec::new(),
            committed: false,
            poisoned: false,
        })
    }

    pub fn poll(&mut self) -> Option<DeltaBatch> {
        self.pending.pop_front()
    }

    pub fn bootstrap(&self) -> RuntimeBootstrap {
        let stream_replica_id = self
            .remote_authority
            .clone()
            .unwrap_or_else(|| self.replica_id.clone());
        let mut seen_batches: Vec<_> = self.seen_batches.values().cloned().collect();
        seen_batches.sort_by(|a, b| {
            a.replica_id
                .cmp(&b.replica_id)
                .then(a.to_seq.cmp(&b.to_seq))
        });

        RuntimeBootstrap {
            seq: self.seq,
            stream_replica_id,
            remote_authority: self.remote_authority.clone(),
            local_authority_established: self.local_authority_established,
            pending: self.pending.iter().cloned().collect(),
            seen_batches,
        }
    }

    pub fn apply_remote<S>(&mut self, state: &mut S, batch: DeltaBatch) -> Result<(), SyncError>
    where
        S: ApplyPath + Clone,
    {
        self.validate_remote_batch(&batch)?;
        let batch_key = batch.batch_key();

        if self.local_authority_established && self.remote_authority.is_none() {
            return Err(SyncError::RoleConflict {
                local_replica_id: self.replica_id.clone(),
                remote_replica_id: batch.replica_id.clone(),
            });
        }

        if let Some(authority) = &self.remote_authority {
            if authority != &batch.replica_id {
                return Err(SyncError::AuthorityMismatch {
                    expected: authority.clone(),
                    actual: batch.replica_id.clone(),
                });
            }
        }

        if batch.from_seq > self.seq {
            return Err(SyncError::GapDetected {
                replica_id: batch.replica_id.clone(),
                expected_from: self.seq,
                actual_from: batch.from_seq,
            });
        }

        if batch.from_seq < self.seq {
            return if let Some(previous) = self.seen_batches.get(&batch_key) {
                if previous.fingerprint == batch.fingerprint() {
                    Ok(())
                } else {
                    Err(SyncError::ConflictingReplay {
                        replica_id: batch.replica_id.clone(),
                        to_seq: batch.to_seq,
                    })
                }
            } else {
                Err(SyncError::StaleBatch {
                    replica_id: batch.replica_id.clone(),
                    local_seq: self.seq,
                    from_seq: batch.from_seq,
                    to_seq: batch.to_seq,
                })
            };
        }

        // Authoritative-writer v1 keeps remote apply atomic by cloning the full
        // materialized state before the batch. This is intentionally simple for
        // Chunk 1 and trades memory/copy cost for explicit rollback semantics.
        let original_state = state.clone();
        for change in &batch.changes {
            if let Err(error) = state.apply_path(change.path.as_slice(), &change.op) {
                *state = original_state;
                return Err(error);
            }
        }

        self.seq = batch.to_seq;
        self.remember_batch(batch_key, batch.clone());
        self.remote_authority = Some(batch.replica_id.clone());

        Ok(())
    }

    fn validate_remote_batch(&self, batch: &DeltaBatch) -> Result<(), SyncError> {
        if batch.changes.is_empty() {
            return Err(SyncError::EmptyBatch {
                replica_id: batch.replica_id.clone(),
            });
        }

        if batch.to_seq != batch.from_seq + 1 {
            return Err(SyncError::InvalidBatchSequence {
                replica_id: batch.replica_id.clone(),
                from_seq: batch.from_seq,
                to_seq: batch.to_seq,
            });
        }

        Ok(())
    }

    fn remember_batch(&mut self, batch_key: (ReplicaId, u64), batch: DeltaBatch) {
        self.seen_batches.insert(batch_key, batch.proof());
    }
}

pub struct BatchTx<'a> {
    ctx: &'a mut ChangeCtx,
    from_seq: u64,
    changes: Vec<ChangeEnvelope>,
    committed: bool,
    poisoned: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeState<T> {
    ctx: ChangeCtx,
    state: T,
}

impl<T> RuntimeState<T>
where
    T: SyncableState,
{
    pub fn new(replica_id: impl Into<ReplicaId>, mut state: T) -> Self {
        if should_rebind_root::<T>() {
            state.rebind_paths(crate::SyncPath::default());
        }
        Self {
            ctx: ChangeCtx::new(replica_id),
            state,
        }
    }

    pub fn restore(
        replica_id: impl Into<ReplicaId>,
        mut state: T,
        snapshot: SnapshotBundle<T::Snapshot>,
        bootstrap: RuntimeBootstrap,
    ) -> Result<Self, SyncError>
    where
        T::Snapshot: PartialEq,
    {
        if should_rebind_root::<T>() {
            state.rebind_paths(crate::SyncPath::default());
        }

        if snapshot.seq != bootstrap.seq || snapshot.replica_id != bootstrap.stream_replica_id {
            return Err(SyncError::BootstrapMismatch {
                snapshot_replica_id: snapshot.replica_id,
                snapshot_seq: snapshot.seq,
                bootstrap_replica_id: bootstrap.stream_replica_id,
                bootstrap_seq: bootstrap.seq,
            });
        }

        if state.snapshot() != snapshot.snapshot {
            return Err(SyncError::SnapshotStateMismatch);
        }

        Ok(Self {
            ctx: ChangeCtx::restore(replica_id, bootstrap)?,
            state,
        })
    }

    pub fn state(&self) -> &T {
        &self.state
    }

    pub fn begin_batch(&mut self) -> Result<BatchTx<'_>, SyncError> {
        self.ctx.begin_batch()
    }

    pub fn with_batch<R>(
        &mut self,
        f: impl FnOnce(&mut T, &mut BatchTx<'_>) -> Result<R, SyncError>,
    ) -> Result<(R, Option<DeltaBatch>), SyncError>
    where
        T: Clone,
    {
        let original_state = self.state.clone();
        let mut batch = self.ctx.begin_batch()?;
        let result = match f(&mut self.state, &mut batch) {
            Ok(result) => result,
            Err(error) => {
                self.state = original_state;
                batch.poison();
                return Err(error);
            }
        };
        let committed = match batch.commit() {
            Ok(committed) => committed,
            Err(error) => {
                self.state = original_state;
                return Err(error);
            }
        };
        Ok((result, committed))
    }

    pub fn poll(&mut self) -> Option<DeltaBatch> {
        self.ctx.poll()
    }

    pub fn current_seq(&self) -> u64 {
        self.ctx.current_seq()
    }

    pub fn snapshot(&self) -> SnapshotBundle<T::Snapshot> {
        let replica_id = self
            .ctx
            .remote_authority
            .clone()
            .unwrap_or_else(|| self.ctx.replica_id.clone());

        SnapshotBundle {
            replica_id,
            seq: self.ctx.seq,
            snapshot: self.state.snapshot(),
        }
    }

    pub fn apply_remote(&mut self, batch: DeltaBatch) -> Result<(), SyncError>
    where
        T: Clone,
    {
        self.ctx.apply_remote(&mut self.state, batch)
    }

    pub fn bootstrap(&self) -> RuntimeBootstrap {
        self.ctx.bootstrap()
    }
}

fn should_rebind_root<T>() -> bool
where
    T: SyncableState,
{
    T::should_rebind_root()
}

impl<T> SyncRuntime for RuntimeState<T>
where
    T: SyncableState + Clone,
{
    fn current_seq(&self) -> u64 {
        self.ctx.current_seq()
    }

    fn snapshot_bundle(&self) -> SnapshotBundle<Self::Snapshot> {
        RuntimeState::snapshot(self)
    }

    fn poll_delta(&mut self) -> Option<DeltaBatch> {
        self.poll()
    }

    fn apply_remote(&mut self, batch: DeltaBatch) -> Result<(), SyncError> {
        self.ctx.apply_remote(&mut self.state, batch)
    }
}

impl<T> SyncableState for RuntimeState<T>
where
    T: SyncableState,
{
    type Snapshot = T::Snapshot;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.snapshot()
    }

    fn schema() -> crate::StateSchema
    where
        Self: Sized,
    {
        T::schema()
    }
}

impl<T> ApplyPath for RuntimeState<T>
where
    T: SyncableState,
{
    fn apply_path(
        &mut self,
        path: &[crate::PathSegment],
        op: &crate::ChangeOp,
    ) -> Result<(), SyncError> {
        self.state.apply_path(path, op)
    }
}

impl<'a> BatchTx<'a> {
    pub fn push(&mut self, change: ChangeEnvelope) {
        self.changes.push(change);
    }

    pub(crate) fn poison(&mut self) {
        self.poisoned = true;
    }

    pub fn commit(mut self) -> Result<Option<DeltaBatch>, SyncError> {
        if self.poisoned {
            self.committed = true;
            return Err(SyncError::BatchAborted);
        }

        self.committed = true;
        if self.changes.is_empty() {
            return Ok(None);
        }

        let changes = core::mem::take(&mut self.changes);

        let to_seq = self.from_seq + 1;
        let batch = DeltaBatch::new(self.ctx.replica_id.clone(), self.from_seq, to_seq, changes);

        self.ctx.seq = to_seq;
        self.ctx.local_authority_established = true;
        self.ctx.pending.push_back(batch.clone());
        self.ctx.remember_batch(batch.batch_key(), batch.clone());

        Ok(Some(batch))
    }
}

fn validate_bootstrap(
    local_replica_id: &ReplicaId,
    bootstrap: &RuntimeBootstrap,
) -> Result<(), SyncError> {
    if let Some(remote_authority) = &bootstrap.remote_authority {
        if remote_authority != &bootstrap.stream_replica_id {
            return Err(SyncError::InvalidBootstrap {
                reason: "remote_authority must match stream_replica_id".into(),
            });
        }

        if bootstrap.local_authority_established {
            return Err(SyncError::InvalidBootstrap {
                reason: "remote_authority and local_authority_established cannot both be set"
                    .into(),
            });
        }

        if !bootstrap.pending.is_empty() {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending local batches cannot coexist with remote authority".into(),
            });
        }
    } else if bootstrap.stream_replica_id != *local_replica_id {
        return Err(SyncError::InvalidBootstrap {
            reason: "stream_replica_id must match local replica when no remote authority exists"
                .into(),
        });
    }

    let mut pending = bootstrap.pending.clone();
    pending.sort_by_key(|batch| batch.from_seq);

    for batch in &pending {
        if batch.replica_id != *local_replica_id {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending batch replica must match local replica".into(),
            });
        }

        if batch.to_seq != batch.from_seq + 1 {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending batch must be a unit-step sequence".into(),
            });
        }

        if batch.to_seq > bootstrap.seq {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending batch cannot advance beyond bootstrap seq".into(),
            });
        }

        if batch.changes.is_empty() {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending batch cannot be empty".into(),
            });
        }
    }

    if let Some(first_pending) = pending.first() {
        let mut expected_from = first_pending.from_seq;

        for batch in &pending {
            if batch.from_seq != expected_from {
                return Err(SyncError::InvalidBootstrap {
                    reason: "pending batches must form one contiguous sequence".into(),
                });
            }

            expected_from = batch.to_seq;
        }

        if expected_from != bootstrap.seq {
            return Err(SyncError::InvalidBootstrap {
                reason: "pending batches must end at bootstrap seq".into(),
            });
        }
    }

    let mut seen_keys = HashMap::new();
    for proof in &bootstrap.seen_batches {
        let key = (proof.replica_id.clone(), proof.to_seq);
        if let Some(existing_fingerprint) = seen_keys.insert(key, proof.fingerprint.clone()) {
            if existing_fingerprint != proof.fingerprint {
                return Err(SyncError::InvalidBootstrap {
                    reason: "seen batch proofs cannot contain conflicting duplicate keys".into(),
                });
            }

            return Err(SyncError::InvalidBootstrap {
                reason: "seen batch proofs cannot contain duplicate keys".into(),
            });
        }

        if proof.to_seq > bootstrap.seq {
            return Err(SyncError::InvalidBootstrap {
                reason: "seen batch cannot advance beyond bootstrap seq".into(),
            });
        }

        match &bootstrap.remote_authority {
            Some(remote_authority) if &proof.replica_id != remote_authority => {
                return Err(SyncError::InvalidBootstrap {
                    reason: "seen batch replica must match remote authority".into(),
                });
            }
            None if bootstrap.local_authority_established
                && proof.replica_id != *local_replica_id =>
            {
                return Err(SyncError::InvalidBootstrap {
                    reason: "seen batch replica must match local replica under local authority"
                        .into(),
                });
            }
            None if !bootstrap.local_authority_established
                && proof.replica_id != *local_replica_id =>
            {
                return Err(SyncError::InvalidBootstrap {
                    reason: "seen batch replica must match local replica before remote authority is established".into(),
                });
            }
            _ => {}
        }
    }

    Ok(())
}
