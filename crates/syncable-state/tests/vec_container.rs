use std::collections::BTreeMap;

use syncable_state::{
    ApplyChildPath, ApplyPath, BatchTx, ChangeCtx, ChangeEnvelope, ChangeOp, DeltaBatch, FieldKind,
    FieldSchema, ListOp, PathSegment, RuntimeState, SnapshotCodec, SnapshotValue, StableId,
    StateSchema, StringOp, SyncContainer, SyncError, SyncPath, SyncableState, SyncableString,
    SyncableVec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoItem {
    id: String,
    title: SyncableString,
}

impl TodoItem {
    fn new(list_path: &SyncPath, id: impl Into<String>, title: impl Into<String>) -> Self {
        let id = id.into();
        let mut path = list_path.clone().into_vec();
        path.push(PathSegment::Id(id.clone()));
        path.push(PathSegment::Field("title".into()));
        Self {
            id,
            title: SyncableString::new(SyncPath::new(path), title),
        }
    }

    fn rename(
        &mut self,
        batch: &mut BatchTx<'_>,
        title: impl Into<String>,
    ) -> Result<(), SyncError> {
        self.title.set(batch, title.into())
    }
}

impl StableId for TodoItem {
    fn stable_id(&self) -> &str {
        &self.id
    }
}

impl ApplyPath for TodoItem {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        self.apply_child_path(path, op)
    }
}

impl ApplyChildPath for TodoItem {
    fn apply_child_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path {
            [PathSegment::Field(field), tail @ ..] if field == "title" => {
                self.title.apply_path_tail(tail, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for TodoItem {
    type Snapshot = BTreeMap<String, SnapshotValue>;

    fn snapshot(&self) -> Self::Snapshot {
        BTreeMap::from([
            ("id".into(), SnapshotValue::String(self.id.clone())),
            (
                "title".into(),
                SnapshotValue::String(self.title.value().to_string()),
            ),
        ])
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![
            FieldSchema {
                name: "id".into(),
                kind: FieldKind::String,
            },
            FieldSchema {
                name: "title".into(),
                kind: FieldKind::String,
            },
        ])
    }
}

impl SnapshotCodec for TodoItem {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue {
        SnapshotValue::Map(snapshot)
    }

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError> {
        match value {
            SnapshotValue::Map(fields) => {
                let id = match fields.get("id") {
                    Some(SnapshotValue::String(id)) => id.clone(),
                    _ => return Err(SyncError::InvalidSnapshotValue),
                };
                let title = match fields.get("title") {
                    Some(SnapshotValue::String(title)) => title.clone(),
                    _ => return Err(SyncError::InvalidSnapshotValue),
                };
                let mut title_path = root_path.into_vec();
                title_path.push(PathSegment::Field("title".into()));
                Ok(Self {
                    id,
                    title: SyncableString::new(SyncPath::new(title_path), title),
                })
            }
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}

#[test]
fn insert_emits_spec_insert_shape_and_snapshot_keeps_stable_order() {
    let mut list = SyncableVec::new(SyncPath::from_field("items"));
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    list.insert(&mut batch, TodoItem::new(list.root_path(), "a", "first"))
        .unwrap();
    list.insert(&mut batch, TodoItem::new(list.root_path(), "b", "second"))
        .unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(
        committed.changes,
        vec![
            ChangeEnvelope::new(
                SyncPath::from_field("items"),
                ChangeOp::List(ListOp::Insert {
                    id: "a".into(),
                    after: None,
                    value: SnapshotValue::Map(BTreeMap::from([
                        ("id".into(), SnapshotValue::String("a".into())),
                        ("title".into(), SnapshotValue::String("first".into())),
                    ])),
                }),
            ),
            ChangeEnvelope::new(
                SyncPath::from_field("items"),
                ChangeOp::List(ListOp::Insert {
                    id: "b".into(),
                    after: Some("a".into()),
                    value: SnapshotValue::Map(BTreeMap::from([
                        ("id".into(), SnapshotValue::String("b".into())),
                        ("title".into(), SnapshotValue::String("second".into())),
                    ])),
                }),
            ),
        ]
    );
    assert_eq!(
        list.snapshot(),
        vec![
            BTreeMap::from([
                ("id".into(), SnapshotValue::String("a".into())),
                ("title".into(), SnapshotValue::String("first".into())),
            ]),
            BTreeMap::from([
                ("id".into(), SnapshotValue::String("b".into())),
                ("title".into(), SnapshotValue::String("second".into())),
            ]),
        ]
    );
}

#[test]
fn delete_by_id_emits_delete_and_get_mut_uses_stable_identity() {
    let mut list = SyncableVec::from_items(
        SyncPath::from_field("items"),
        vec![
            TodoItem::new(&SyncPath::from_field("items"), "a", "first"),
            TodoItem::new(&SyncPath::from_field("items"), "b", "second"),
        ],
    )
    .unwrap();
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    list.get_mut("b")
        .unwrap()
        .rename(&mut batch, "second-updated")
        .unwrap();
    list.delete(&mut batch, "a").unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(list.get_mut("a"), None);
    assert_eq!(list.get_mut("b").unwrap().title.value(), "second-updated");
    assert_eq!(
        committed.changes[1],
        ChangeEnvelope::new(
            SyncPath::from_field("items"),
            ChangeOp::List(ListOp::Delete { id: "a".into() }),
        )
    );
}

#[test]
fn insert_canonicalizes_child_root_for_future_nested_deltas() {
    let mut list = SyncableVec::new(SyncPath::from_field("items"));
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    list.insert(
        &mut batch,
        TodoItem::new(&SyncPath::from_field("stale-items"), "a", "first"),
    )
    .unwrap();
    batch.commit().unwrap().unwrap();

    let mut rename_batch = ctx.begin_batch().unwrap();
    list.get_mut("a")
        .unwrap()
        .rename(&mut rename_batch, "updated")
        .unwrap();
    let committed = rename_batch.commit().unwrap().unwrap();

    assert_eq!(
        committed.changes,
        vec![ChangeEnvelope::new(
            SyncPath::new(vec![
                PathSegment::Field("items".into()),
                PathSegment::Id("a".into()),
                PathSegment::Field("title".into()),
            ]),
            ChangeOp::String(StringOp::Set("updated".into())),
        )]
    );
}

#[test]
fn from_items_canonicalizes_child_roots_for_future_nested_deltas() {
    let mut list = SyncableVec::from_items(
        SyncPath::from_field("items"),
        vec![TodoItem::new(
            &SyncPath::from_field("stale-items"),
            "a",
            "first",
        )],
    )
    .unwrap();
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    list.get_mut("a")
        .unwrap()
        .rename(&mut batch, "updated")
        .unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(
        committed.changes,
        vec![ChangeEnvelope::new(
            SyncPath::new(vec![
                PathSegment::Field("items".into()),
                PathSegment::Id("a".into()),
                PathSegment::Field("title".into()),
            ]),
            ChangeOp::String(StringOp::Set("updated".into())),
        )]
    );
}

#[test]
fn remote_insert_rejects_unknown_extra_snapshot_fields() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableVec::<TodoItem>::new(SyncPath::from_field("items")),
    );

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("items"),
                ChangeOp::List(ListOp::Insert {
                    id: "a".into(),
                    after: None,
                    value: SnapshotValue::Map(BTreeMap::from([
                        ("id".into(), SnapshotValue::String("a".into())),
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
fn child_routing_passes_tail_only_to_resolved_vec_item() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableVec::from_items(
            SyncPath::from_field("items"),
            vec![TodoItem::new(&SyncPath::from_field("items"), "a", "before")],
        )
        .unwrap(),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::new(vec![
                    PathSegment::Field("items".into()),
                    PathSegment::Id("a".into()),
                    PathSegment::Field("title".into()),
                ]),
                ChangeOp::String(StringOp::Set("after".into())),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().get("a").unwrap().title.value(), "after");
}

#[test]
fn from_items_returns_error_instead_of_panicking_on_duplicate_id() {
    let error = SyncableVec::from_items(
        SyncPath::from_field("items"),
        vec![
            TodoItem::new(&SyncPath::from_field("items"), "a", "first"),
            TodoItem::new(&SyncPath::from_field("items"), "a", "second"),
        ],
    )
    .unwrap_err();

    assert_eq!(error, SyncError::DuplicateStableId { id: "a".into() });
}

#[test]
fn nested_child_updates_route_through_stable_id_paths() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableVec::from_items(
            SyncPath::from_field("items"),
            vec![TodoItem::new(&SyncPath::from_field("items"), "a", "before")],
        )
        .unwrap(),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::new(vec![
                    PathSegment::Field("items".into()),
                    PathSegment::Id("a".into()),
                    PathSegment::Field("title".into()),
                ]),
                ChangeOp::String(StringOp::Set("after".into())),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().get("a").unwrap().title.value(), "after");
}

#[test]
fn remote_insert_materializes_child_from_snapshot_payload_in_one_step() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableVec::<TodoItem>::new(SyncPath::from_field("items")),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("items"),
                ChangeOp::List(ListOp::Insert {
                    id: "a".into(),
                    after: None,
                    value: SnapshotValue::Map(BTreeMap::from([
                        ("id".into(), SnapshotValue::String("a".into())),
                        ("title".into(), SnapshotValue::String("first".into())),
                    ])),
                }),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().get("a").unwrap().title.value(), "first");
    assert_eq!(runtime.snapshot().snapshot.len(), 1);
}
