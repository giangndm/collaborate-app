use syncable_state::SyncableString;

#[derive(syncable_state_derive::SyncableState)]
struct WithAttrState {
    #[sync(with = custom_codec)]
    title: SyncableString,
}

fn main() {}
