#![allow(dead_code)]
use std::collections::BTreeMap;

use syncable_state::{
    ApplyChildPath, ApplyPath, ChangeEnvelope, ChangeOp, DeltaBatch, FieldKind, FieldSchema, MapOp,
    PathSegment, RuntimeState, SnapshotCodec, SnapshotValue, StateSchema, StringOp, SyncContainer,
    SyncError, SyncPath, SyncableMap, SyncableState, SyncableString,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoteValue {
    id: String,
    title: SyncableString,
}

impl NoteValue {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into().into(),
        }
    }

    fn rename(&mut self, title: impl Into<String>) -> Result<(), SyncError> {
        self.title.set(title.into())
    }
}

impl ApplyPath for NoteValue {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        self.apply_child_path(path, op)
    }
}

impl ApplyChildPath for NoteValue {
    fn apply_child_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path {
            [PathSegment::Field(field), tail @ ..] if field == "title" => {
                self.title.apply_path_tail(tail, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for NoteValue {
    type Snapshot = BTreeMap<String, SnapshotValue>;

    fn snapshot(&self) -> Self::Snapshot {
        BTreeMap::from([(
            "title".into(),
            SnapshotValue::String(self.title.value().to_string()),
        )])
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![FieldSchema {
            name: "title".into(),
            kind: FieldKind::String,
        }])
    }

    fn rebind_paths(&mut self, root_path: SyncPath, tracker: Option<syncable_state::EventTracker>) {
        let mut child_root = root_path.clone().into_vec();
        child_root.push(PathSegment::Field("title".into()));
        self.title.rebind_paths(SyncPath::new(child_root), tracker);
    }
}

impl SnapshotCodec for NoteValue {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue {
        SnapshotValue::Map(snapshot)
    }

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError> {
        match value {
            SnapshotValue::Map(fields) => match fields.get("title") {
                Some(SnapshotValue::String(title)) => {
                    let mut state = Self {
                        id: "unknown".into(),
                        title: SyncableString::from(title.clone()),
                    };
                    state.rebind_paths(root_path, None);
                    Ok(state)
                }
                _ => Err(SyncError::InvalidSnapshotValue),
            },
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}

#[test]
fn insert_replace_remove_emit_explicit_map_ops_and_snapshot_is_deterministic() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut map = SyncableMap::<String, NoteValue>::default();
    map.rebind_paths(SyncPath::from_field("notes"), Some(tracker.clone()));

    map.insert("b".to_string(), NoteValue::new("b", "second"))
        .unwrap();
    map.insert("a".to_string(), NoteValue::new("a", "first"))
        .unwrap();
    map.replace("a".to_string(), NoteValue::new("a", "first-updated"))
        .unwrap();
    map.remove(&"b".to_string()).unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        changes,
        vec![
            ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Insert {
                    key: "b".into(),
                    value: SnapshotValue::Map(BTreeMap::from([(
                        "title".into(),
                        SnapshotValue::String("second".into()),
                    )])),
                }),
            ),
            ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Insert {
                    key: "a".into(),
                    value: SnapshotValue::Map(BTreeMap::from([(
                        "title".into(),
                        SnapshotValue::String("first".into()),
                    )])),
                }),
            ),
            ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Replace {
                    key: "a".into(),
                    value: SnapshotValue::Map(BTreeMap::from([(
                        "title".into(),
                        SnapshotValue::String("first-updated".into()),
                    )])),
                }),
            ),
            ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Remove { key: "b".into() }),
            ),
        ]
    );
    assert_eq!(
        map.snapshot(),
        BTreeMap::from([(
            "a".into(),
            BTreeMap::from([(
                "title".into(),
                SnapshotValue::String("first-updated".into()),
            )]),
        )])
    );
}

#[test]
fn replace_canonicalizes_child_root_for_future_nested_deltas() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut map =
        SyncableMap::from_entries([("a".to_string(), NoteValue::new("a", "before"))]).unwrap();
    map.rebind_paths(SyncPath::from_field("notes"), Some(tracker.clone()));

    map.replace("a".to_string(), NoteValue::new("a", "after"))
        .unwrap();

    tracker.borrow_mut().clear();

    map.get_mut(&"a".to_string())
        .unwrap()
        .title
        .set("final")
        .unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        changes,
        vec![ChangeEnvelope::new(
            SyncPath::new(vec![
                PathSegment::Field("notes".into()),
                PathSegment::Key("a".into()),
                PathSegment::Field("title".into()),
            ]),
            ChangeOp::String(StringOp::Set("final".into())),
        )]
    );
}

#[test]
fn from_entries_canonicalizes_child_roots_for_future_nested_deltas() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut map =
        SyncableMap::from_entries([("a".to_string(), NoteValue::new("a", "before"))]).unwrap();
    map.rebind_paths(SyncPath::from_field("notes"), Some(tracker.clone()));

    map.get_mut(&"a".to_string())
        .unwrap()
        .title
        .set("after")
        .unwrap();
    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        changes,
        vec![ChangeEnvelope::new(
            SyncPath::new(vec![
                PathSegment::Field("notes".into()),
                PathSegment::Key("a".into()),
                PathSegment::Field("title".into()),
            ]),
            ChangeOp::String(StringOp::Set("after".into())),
        )]
    );
}

#[test]
fn nested_child_routing_uses_key_segments() {
    let mut runtime = RuntimeState::new("local", {
        let mut m =
            SyncableMap::from_entries([("a".to_string(), NoteValue::new("a", "before"))]).unwrap();
        m.rebind_paths(SyncPath::from_field("notes"), None);
        m
    });

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::new(vec![
                    PathSegment::Field("notes".into()),
                    PathSegment::Key("a".into()),
                    PathSegment::Field("title".into()),
                ]),
                ChangeOp::String(StringOp::Set("after".into())),
            )],
        ))
        .unwrap();

    assert_eq!(
        runtime.state().get(&"a".to_string()).unwrap().title.value(),
        "after"
    );
}

#[test]
fn remote_insert_and_replace_use_full_snapshot_payload_with_schema_validation() {
    let mut runtime = RuntimeState::new("local", {
        let mut m = SyncableMap::<String, NoteValue>::default();
        m.rebind_paths(SyncPath::from_field("notes"), None);
        m
    });

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Insert {
                    key: "a".into(),
                    value: SnapshotValue::Map(BTreeMap::from([(
                        "title".into(),
                        SnapshotValue::String("first".into()),
                    )])),
                }),
            )],
        ))
        .unwrap();

    assert_eq!(
        runtime.state().get(&"a".to_string()).unwrap().title.value(),
        "first"
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            1,
            2,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Replace {
                    key: "a".into(),
                    value: SnapshotValue::Map(BTreeMap::from([(
                        "title".into(),
                        SnapshotValue::String("updated".into()),
                    )])),
                }),
            )],
        ))
        .unwrap();

    assert_eq!(
        runtime.state().get(&"a".to_string()).unwrap().title.value(),
        "updated"
    );

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            2,
            3,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Insert {
                    key: "b".into(),
                    value: SnapshotValue::String("wrong-shape".into()),
                }),
            )],
        ))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidSnapshotValue);
    assert!(runtime.state().get(&"b".to_string()).is_none());
}

#[test]
fn remote_insert_rejects_unknown_extra_snapshot_fields() {
    let mut runtime = RuntimeState::new("local", {
        let mut m = SyncableMap::<String, NoteValue>::default();
        m.rebind_paths(SyncPath::from_field("notes"), None);
        m
    });

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Insert {
                    key: "a".into(),
                    value: SnapshotValue::Map(BTreeMap::from([
                        ("title".into(), SnapshotValue::String("first".into())),
                        ("extra".into(), SnapshotValue::String("nope".into())),
                    ])),
                }),
            )],
        ))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidSnapshotValue);
}

#[test]
fn remote_remove_fails_when_key_does_not_exist() {
    let mut runtime = RuntimeState::new("local", {
        let mut m = SyncableMap::<String, NoteValue>::default();
        m.rebind_paths(SyncPath::from_field("notes"), None);
        m
    });

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("notes"),
                ChangeOp::Map(MapOp::Remove {
                    key: "missing".into(),
                }),
            )],
        ))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidPath);
}

#[test]
fn child_routing_passes_tail_only_to_resolved_map_value() {
    let mut runtime = RuntimeState::new("local", {
        let mut m =
            SyncableMap::from_entries([("a".to_string(), NoteValue::new("a", "before"))]).unwrap();
        m.rebind_paths(SyncPath::from_field("notes"), None);
        m
    });

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::new(vec![
                    PathSegment::Field("notes".into()),
                    PathSegment::Key("a".into()),
                    PathSegment::Field("title".into()),
                ]),
                ChangeOp::String(StringOp::Set("after".into())),
            )],
        ))
        .unwrap();

    assert_eq!(
        runtime.state().get(&"a".to_string()).unwrap().title.value(),
        "after"
    );
}

#[test]
fn map_supports_scalar_syncable_string_values() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut map = SyncableMap::<String, SyncableString>::default();
    map.rebind_paths(SyncPath::from_field("labels"), Some(tracker.clone()));

    map.insert("a".to_string(), SyncableString::from("first"))
        .unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        changes,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("labels"),
            ChangeOp::Map(MapOp::Insert {
                key: "a".into(),
                value: SnapshotValue::String("first".into()),
            }),
        )]
    );

    map.get_mut(&"a".to_string())
        .unwrap()
        .set("updated")
        .unwrap();
    let rename = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        rename,
        vec![ChangeEnvelope::new(
            SyncPath::new(vec![
                PathSegment::Field("labels".into()),
                PathSegment::Key("a".into()),
            ]),
            ChangeOp::String(StringOp::Set("updated".into())),
        )]
    );
}

#[test]
fn from_entries_returns_error_instead_of_panicking_on_duplicate_key() {
    let error = SyncableMap::from_entries([
        ("a".to_string(), NoteValue::new("a", "first")),
        ("a".to_string(), NoteValue::new("a", "second")),
    ])
    .unwrap_err();

    assert_eq!(error, SyncError::DuplicateMapKey { key: "a".into() });
}
