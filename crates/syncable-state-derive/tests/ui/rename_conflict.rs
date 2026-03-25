use syncable_state::SyncableString;

#[derive(syncable_state_derive::SyncableState)]
struct RenameConflictState {
    #[sync(rename = "title")]
    name: SyncableString,
    title: SyncableString,
}

fn main() {}
