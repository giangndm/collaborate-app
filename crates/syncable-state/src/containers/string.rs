use crate::{
    ApplyPath, ChangeEnvelope, ChangeOp, FieldSchema, PathSegment, SnapshotCodec, SnapshotValue,
    StateSchema, StringContainer, StringOp, SyncContainer, SyncError, SyncPath, SyncableState,
};

/// A synchronization container for scalar text structures.
///
/// `SyncableString` holds a single string value and replicates full-value
/// overwrites. It is optimized for short labels, IDs, or identifiers where
/// setting the value atomically is preferred to character-by-character editing.
///
/// # Example
///
/// ```rust
/// # use syncable_state::{SyncableState, SyncableString, SyncPath, RuntimeState};
/// let mut title = SyncableString::from("initial");
/// let mut runtime = RuntimeState::new("node-A", title);
///
/// runtime.with_batch(|state, batch| {
///     state.set(batch, "updated")?;
///     Ok::<(), syncable_state::SyncError>(())
/// }).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableString {
    root_path: SyncPath,
    tracker: Option<crate::EventTracker>,
    value: String,
}

impl SyncableString {
    pub(crate) fn new(root_path: SyncPath, value: impl Into<String>) -> Self {
        Self {
            root_path,
            tracker: None,
            value: value.into(),
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn set(&mut self, value: impl Into<String>) -> Result<(), SyncError> {
        StringContainer::set(self, value.into())
    }

    pub fn clear(&mut self) -> Result<(), SyncError> {
        StringContainer::clear(self)
    }

    fn apply_op(&mut self, op: &StringOp) {
        match op {
            StringOp::Set(value) => self.value = value.clone(),
            StringOp::Clear => self.value.clear(),
        }
    }
}

impl SyncContainer for SyncableString {
    type Snapshot = String;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.value.clone()
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::String(op)) => {
                self.apply_op(op);
                Ok(())
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl StringContainer for SyncableString {
    fn value(&self) -> &str {
        &self.value
    }

    fn set(&mut self, value: String) -> Result<(), SyncError> {
        self.value = value.clone();
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::String(StringOp::Set(value)),
            ));
        }
        Ok(())
    }

    fn clear(&mut self) -> Result<(), SyncError> {
        self.value.clear();
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::String(StringOp::Clear),
            ));
        }
        Ok(())
    }
}

impl ApplyPath for SyncableString {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl SyncableState for SyncableString {
    type Snapshot = String;

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
            kind: crate::FieldKind::String,
        }])
    }
}

impl SnapshotCodec for SyncableString {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue {
        SnapshotValue::String(snapshot)
    }

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError> {
        match value {
            SnapshotValue::String(value) => Ok(Self::new(root_path, value)),
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}

impl Default for SyncableString {
    fn default() -> Self {
        Self::new(SyncPath::default(), "")
    }
}

impl From<&str> for SyncableString {
    fn from(s: &str) -> Self {
        Self::new(SyncPath::default(), s)
    }
}

impl From<String> for SyncableString {
    fn from(s: String) -> Self {
        Self::new(SyncPath::default(), s)
    }
}
