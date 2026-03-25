use crate::{
    ChangeEnvelope, ChangeOp, DeltaBatch, PathSegment, SnapshotBundle, SnapshotValue, StateSchema,
    SyncError, SyncPath,
};
use std::cell::RefCell;
use std::rc::Rc;

pub type EventTracker = Rc<RefCell<Vec<ChangeEnvelope>>>;

pub trait ApplyPath {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError>;
}

pub trait ApplyChildPath {
    fn apply_child_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError>;
}

pub trait SyncableState: ApplyPath {
    type Snapshot: Clone;

    fn snapshot(&self) -> Self::Snapshot;

    fn rebind_paths(&mut self, _root_path: SyncPath, _tracker: Option<EventTracker>) {}

    fn is_scalar_value() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn should_rebind_root() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn schema() -> StateSchema
    where
        Self: Sized;
}

pub trait SyncContainer: ApplyPath {
    type Snapshot: Clone;

    fn root_path(&self) -> &SyncPath;

    fn snapshot_value(&self) -> Self::Snapshot;

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError>;
}

pub trait StringContainer: SyncContainer<Snapshot = String> {
    fn value(&self) -> &str;

    fn set(&mut self, value: String) -> Result<(), SyncError>;

    fn clear(&mut self) -> Result<(), SyncError>;
}

pub trait CounterContainer: SyncContainer<Snapshot = i64> {
    fn value(&self) -> i64;

    fn increment(&mut self, amount: i64) -> Result<(), SyncError>;

    fn decrement(&mut self, amount: i64) -> Result<(), SyncError>;

    fn multiply(&mut self, amount: i64) -> Result<(), SyncError>;
}

pub trait TextContainer: SyncContainer<Snapshot = String> {
    fn value(&self) -> &str;

    fn splice(&mut self, index: usize, delete: usize, insert: String) -> Result<(), SyncError>;

    fn clear(&mut self) -> Result<(), SyncError>;
}

pub trait StableId {
    type Id: core::fmt::Display;
    fn stable_id(&self) -> &Self::Id;
}

pub trait SnapshotCodec: SyncableState + Sized {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue;

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError>;
}

pub trait SyncRuntime: SyncableState {
    fn current_seq(&self) -> u64;

    fn snapshot_bundle(&self) -> SnapshotBundle<Self::Snapshot>;

    fn poll_delta(&mut self) -> Option<DeltaBatch>;

    fn apply_remote(&mut self, batch: DeltaBatch) -> Result<(), SyncError>;
}

impl<T> ApplyChildPath for T
where
    T: SyncContainer,
{
    fn apply_child_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        self.apply_path_tail(path, op)
    }
}
