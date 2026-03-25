use syncable_state::{
    ApplyPath, ChangeOp, MapOp, PathSegment, SnapshotCodec, StringOp, SyncPath, SyncableMap,
    SyncableState, SyncableString, SyncableVec,
};

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct RowState {
    #[sync(id)]
    id: String,
    title: SyncableString,
}

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct NoteState {
    title: SyncableString,
}

#[derive(Clone, Debug, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct DocumentState {
    rows: SyncableVec<RowState>,
    notes: SyncableMap<String, NoteState>,
}

fn main() {
    let rows = SyncableVec::from_items(
        vec![RowState {
            id: "a".into(),
            title: SyncableString::from("first"),
        }],
    )
    .unwrap();
    let notes = SyncableMap::from_entries(
        [(
            String::from("left"),
            NoteState {
                title: SyncableString::from("memo"),
            },
        )],
    )
    .unwrap();
    let mut state = DocumentState { rows, notes };

    let snapshot = state.snapshot();
    let encoded = DocumentState::snapshot_to_value(snapshot.clone());
    let restored = DocumentState::snapshot_from_value(SyncPath::default(), encoded).unwrap();

    let restored_snapshot = restored.snapshot();
    assert_eq!(restored_snapshot.rows.len(), snapshot.rows.len());
    assert_eq!(restored_snapshot.rows[0].id, snapshot.rows[0].id);
    assert_eq!(restored_snapshot.rows[0].title, snapshot.rows[0].title);
    assert_eq!(restored_snapshot.notes.len(), snapshot.notes.len());
    assert_eq!(
        restored_snapshot.notes.get("left").unwrap().title,
        snapshot.notes.get("left").unwrap().title
    );

    state
        .apply_path(
            &[
                PathSegment::Field("rows".into()),
                PathSegment::Id("a".into()),
                PathSegment::Field("title".into()),
            ],
            &ChangeOp::String(StringOp::Set("updated".into())),
        )
        .unwrap();
    state
        .apply_path(
            &[
                PathSegment::Field("notes".into()),
                PathSegment::Key("left".into()),
                PathSegment::Field("title".into()),
            ],
            &ChangeOp::String(StringOp::Set("revised".into())),
        )
        .unwrap();
    state
        .apply_path(
            &[PathSegment::Field("notes".into())],
            &ChangeOp::Map(MapOp::Remove { key: "left".into() }),
        )
        .unwrap();

    assert_eq!(state.rows.get("a").unwrap().title.value(), "updated");
    assert!(state.notes.get(&"left".to_string()).is_none());
}
