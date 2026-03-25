use syncable_state::{SyncableString, SyncableVec};

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct RowState {
    title: SyncableString,
}

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct DocumentState {
    rows: SyncableVec<RowState>,
}

fn main() {}
