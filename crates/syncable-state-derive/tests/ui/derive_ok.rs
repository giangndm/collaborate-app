use syncable_state::{
    ApplyPath, ChangeOp, FieldKind, PathSegment, StringOp, SyncPath, SyncableState, SyncableString,
};

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct DemoState {
    #[sync(id)]
    id: String,
    #[sync(rename = "headline")]
    title: SyncableString,
}

fn main() {
    let mut state = DemoState {
        id: "doc-1".into(),
        title: SyncableString::new(SyncPath::from_field("headline"), "hello"),
    };

    let snapshot = state.snapshot();
    assert_eq!(snapshot.id, "doc-1");
    assert_eq!(snapshot.title, "hello");
    assert_eq!(DemoState::schema().fields[0].name, "id");
    assert_eq!(DemoState::schema().fields[0].kind, FieldKind::String);
    assert_eq!(DemoState::schema().fields[1].name, "headline");

    state
        .apply_path(
            &[PathSegment::Field("headline".into())],
            &ChangeOp::String(StringOp::Set("updated".into())),
        )
        .unwrap();

    assert_eq!(state.title.value(), "updated");
}
