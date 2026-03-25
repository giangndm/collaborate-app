use syncable_state::SyncableString;

#[derive(syncable_state_derive::SyncableState)]
struct TupleState(SyncableString);

fn main() {}
