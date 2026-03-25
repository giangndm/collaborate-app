use crate::{
    ApplyPath, ChangeEnvelope, ChangeOp, CounterContainer, CounterOp, FieldSchema, PathSegment,
    SnapshotCodec, SnapshotValue, StateSchema, SyncContainer, SyncError, SyncPath, SyncableState,
};

/// A synchronization container for numerical counts that supports commutative updates.
///
/// `SyncableCounter` allows peers to emit `Increment` or `Decrement` operations,
/// avoiding conflicts that arise when peers overwrite the same scalar value simultaneously.
///
/// # Example
///
/// ```rust
/// # use syncable_state::{SyncableState, SyncableCounter, SyncPath, RuntimeState};
/// let mut score = SyncableCounter::from(0);
/// let mut runtime = RuntimeState::new("node-A", score);
///
/// runtime.with_batch(|state, batch| {
///     state.increment(batch, 10)?;
///     state.decrement(batch, 2)?;
///     Ok::<(), syncable_state::SyncError>(())
/// }).unwrap();
///
/// assert_eq!(runtime.state().value(), 8);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableCounter {
    root_path: SyncPath,
    tracker: Option<crate::EventTracker>,
    value: i64,
}

impl SyncableCounter {
    pub(crate) fn new(root_path: SyncPath, value: i64) -> Self {
        Self {
            root_path,
            tracker: None,
            value,
        }
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn increment(&mut self, amount: i64) -> Result<(), SyncError> {
        CounterContainer::increment(self, amount)
    }

    pub fn decrement(&mut self, amount: i64) -> Result<(), SyncError> {
        CounterContainer::decrement(self, amount)
    }

    fn apply_op(&mut self, op: &CounterOp) -> Result<(), SyncError> {
        match op {
            CounterOp::Increment(amount) => {
                if *amount < 0 {
                    return Err(SyncError::InvalidCounterAmount);
                }
                self.apply_delta(*amount)
            }
            CounterOp::Decrement(amount) => {
                if *amount < 0 {
                    return Err(SyncError::InvalidCounterAmount);
                }
                self.apply_delta(amount.checked_neg().ok_or(SyncError::CounterOverflow {
                    current: self.value,
                    delta: i64::MIN,
                })?)
            }
        }
    }

    fn apply_delta(&mut self, delta: i64) -> Result<(), SyncError> {
        let next = self
            .value
            .checked_add(delta)
            .ok_or(SyncError::CounterOverflow {
                current: self.value,
                delta,
            })?;
        self.value = next;
        Ok(())
    }
}

impl SyncContainer for SyncableCounter {
    type Snapshot = i64;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.value
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::Counter(op)) => self.apply_op(op),
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl CounterContainer for SyncableCounter {
    fn value(&self) -> i64 {
        self.value
    }

    fn increment(&mut self, amount: i64) -> Result<(), SyncError> {
        if amount < 0 {
            return Err(SyncError::InvalidCounterAmount);
        }

        self.apply_delta(amount)?;
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::Counter(CounterOp::Increment(amount)),
            ));
        }
        Ok(())
    }

    fn decrement(&mut self, amount: i64) -> Result<(), SyncError> {
        if amount < 0 {
            return Err(SyncError::InvalidCounterAmount);
        }

        let Some(delta) = amount.checked_neg() else {
            return Err(SyncError::CounterOverflow {
                current: self.value,
                delta: i64::MIN,
            });
        };
        self.apply_delta(delta)?;
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::Counter(CounterOp::Decrement(amount)),
            ));
        }
        Ok(())
    }
}

impl ApplyPath for SyncableCounter {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl SyncableState for SyncableCounter {
    type Snapshot = i64;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_value()
    }

    fn rebind_paths(&mut self, root_path: SyncPath, tracker: Option<crate::EventTracker>) {
        self.root_path = root_path;
        self.tracker = tracker;
    }

    fn is_scalar_value() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn schema() -> StateSchema
    where
        Self: Sized,
    {
        StateSchema::new(vec![FieldSchema {
            name: "root".into(),
            kind: crate::FieldKind::Counter,
        }])
    }
}

impl SnapshotCodec for SyncableCounter {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue {
        SnapshotValue::Counter(snapshot)
    }

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError> {
        match value {
            SnapshotValue::Counter(value) => Ok(Self::new(root_path, value)),
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}

impl Default for SyncableCounter {
    fn default() -> Self {
        Self::new(SyncPath::default(), 0)
    }
}

impl From<i64> for SyncableCounter {
    fn from(value: i64) -> Self {
        Self::new(SyncPath::default(), value)
    }
}
