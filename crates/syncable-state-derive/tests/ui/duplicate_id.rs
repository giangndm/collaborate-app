use syncable_state::SyncableString;

#[derive(syncable_state_derive::SyncableState)]
struct DuplicateIdState {
    #[sync(id)]
    first_id: String,
    #[sync(id)]
    second_id: String,
    title: SyncableString,
}

fn main() {}
