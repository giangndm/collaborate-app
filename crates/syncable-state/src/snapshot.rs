use std::collections::BTreeMap;

use crate::{ChangeEnvelope, ReplicaId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SnapshotValue {
    Null,
    String(String),
    Counter(i64),
    List(Vec<SnapshotValue>),
    Map(BTreeMap<String, SnapshotValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotBundle<T> {
    pub replica_id: ReplicaId,
    pub seq: u64,
    pub snapshot: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBootstrap {
    pub seq: u64,
    pub stream_replica_id: ReplicaId,
    pub remote_authority: Option<ReplicaId>,
    pub local_authority_established: bool,
    pub pending: Vec<DeltaBatch>,
    pub seen_batches: Vec<BatchProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchProof {
    pub replica_id: ReplicaId,
    pub to_seq: u64,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeltaBatch {
    pub replica_id: ReplicaId,
    pub from_seq: u64,
    pub to_seq: u64,
    pub changes: Vec<ChangeEnvelope>,
}

impl DeltaBatch {
    pub fn new(
        replica_id: impl Into<ReplicaId>,
        from_seq: u64,
        to_seq: u64,
        changes: Vec<ChangeEnvelope>,
    ) -> Self {
        Self {
            replica_id: replica_id.into(),
            from_seq,
            to_seq,
            changes,
        }
    }

    pub fn batch_key(&self) -> (ReplicaId, u64) {
        (self.replica_id.clone(), self.to_seq)
    }

    pub fn fingerprint(&self) -> String {
        serde_json::to_string(self).expect("delta batch should serialize for replay proof")
    }

    pub fn proof(&self) -> BatchProof {
        BatchProof {
            replica_id: self.replica_id.clone(),
            to_seq: self.to_seq,
            fingerprint: self.fingerprint(),
        }
    }
}
