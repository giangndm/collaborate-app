use std::collections::BTreeMap;

use syncable_state::{
    ApplyChildPath, ApplyPath, BatchTx, ChangeCtx, ChangeEnvelope, ChangeOp, CounterOp, FieldKind,
    FieldSchema, ListOp, PathSegment, RuntimeState, SnapshotCodec, SnapshotValue, StableId,
    StateSchema, StringOp, SyncContainer, SyncError, SyncPath, SyncRuntime, SyncableCounter,
    SyncableState, SyncableString, SyncableText, SyncableVec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RowState {
    id: String,
    title: SyncableText,
}

impl RowState {
    fn new(list_path: &SyncPath, id: impl Into<String>, title: impl Into<String>) -> Self {
        let id = id.into();
        let mut path = list_path.clone().into_vec();
        path.push(PathSegment::Id(id.clone()));
        path.push(PathSegment::Field("title".into()));
        Self {
            id,
            title: SyncableText::new(SyncPath::new(path), title),
        }
    }
}

impl StableId for RowState {
    type Id = String;
    fn stable_id(&self) -> &Self::Id {
        &self.id
    }
}

impl ApplyPath for RowState {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        self.apply_child_path(path, op)
    }
}

impl ApplyChildPath for RowState {
    fn apply_child_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path {
            [PathSegment::Field(field), tail @ ..] if field == "title" => {
                self.title.apply_path_tail(tail, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for RowState {
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

impl SnapshotCodec for RowState {
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
                    title: SyncableText::new(SyncPath::new(title_path), title),
                })
            }
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocumentState {
    title: SyncableString,
    revision: SyncableCounter,
    rows: SyncableVec<RowState>,
}

impl DocumentState {
    fn new() -> Self {
        let rows_path = SyncPath::from_field("rows");
        Self {
            title: SyncableString::new(SyncPath::from_field("title"), "Doc"),
            revision: SyncableCounter::new(SyncPath::from_field("revision"), 0),
            rows: SyncableVec::from_items(
                rows_path.clone(),
                vec![
                    RowState::new(&rows_path, "a", "First"),
                    RowState::new(&rows_path, "b", "Second"),
                ],
            )
            .unwrap(),
        }
    }

    fn rename(&mut self, batch: &mut BatchTx<'_>, title: &str) -> Result<(), SyncError> {
        self.title.set(batch, title)?;
        self.revision.increment(batch, 1)?;
        Ok(())
    }
}

impl ApplyPath for DocumentState {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path {
            [PathSegment::Field(field), tail @ ..] if field == "title" => {
                self.title.apply_path_tail(tail, op)
            }
            [PathSegment::Field(field), tail @ ..] if field == "revision" => {
                self.revision.apply_path_tail(tail, op)
            }
            [PathSegment::Field(field), tail @ ..] if field == "rows" => {
                self.rows.apply_path_tail(tail, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for DocumentState {
    type Snapshot = (String, i64, Vec<BTreeMap<String, SnapshotValue>>);

    fn snapshot(&self) -> Self::Snapshot {
        (
            self.title.value().to_string(),
            self.revision.value(),
            self.rows.snapshot(),
        )
    }

    fn schema() -> StateSchema {
        StateSchema::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, syncable_state_derive::SyncableState)]
struct DerivedDocumentState {
    #[sync(id)]
    id: String,
    #[sync(rename = "headline")]
    title: SyncableString,
    revision: syncable_state::SyncableCounter,
    #[sync(skip)]
    local_cache: usize,
}

#[test]
fn delete_by_id_emits_list_delete() {
    let mut state = DocumentState::new();
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    state.rows.delete(&mut batch, "a").unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(
        committed.changes,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("rows"),
            ChangeOp::List(ListOp::Delete { id: "a".into() }),
        )]
    );
}

#[test]
fn rename_emits_one_committed_batch_with_title_and_revision_changes() {
    let mut state = DocumentState::new();
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    state.rename(&mut batch, "Renamed").unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(committed.changes.len(), 2);
    assert_eq!(
        committed.changes[0],
        ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::String(StringOp::Set("Renamed".into())),
        )
    );
    assert_eq!(
        committed.changes[1],
        ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )
    );
}

#[test]
fn snapshot_sequence_and_polled_delta_sequence_stay_aligned() {
    let mut runtime = RuntimeState::new("local", DocumentState::new());
    runtime
        .with_batch(|state, batch| {
            state.rename(batch, "Renamed")?;
            Ok(())
        })
        .unwrap();
    runtime
        .with_batch(|state, batch| {
            state.rows.delete(batch, "a")?;
            Ok(())
        })
        .unwrap();

    let snapshot = runtime.snapshot_bundle();
    let emitted = vec![runtime.poll_delta().unwrap(), runtime.poll_delta().unwrap()];
    let mut peer = RuntimeState::new("peer", DocumentState::new());

    for delta in &emitted {
        peer.apply_remote(delta.clone()).unwrap();
    }

    assert_eq!(emitted[0].to_seq, 1);
    assert_eq!(emitted[1].to_seq, 2);
    assert_eq!(snapshot.seq, emitted[1].to_seq);
    assert_eq!(snapshot.snapshot.0, "Renamed");
    assert_eq!(snapshot.snapshot.1, 1);
    assert_eq!(peer.snapshot_bundle(), snapshot);
}

#[test]
fn failed_local_multi_step_batch_rolls_back_state_and_emits_no_delta() {
    let mut runtime = RuntimeState::new("local", DocumentState::new());
    let before = runtime.snapshot_bundle();

    let error = runtime
        .with_batch(|state, batch| {
            state.rename(batch, "Renamed")?;
            state.rows.delete(batch, "missing")?;
            Ok(())
        })
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::StableIdNotFound {
            id: "missing".into()
        }
    );
    assert_eq!(runtime.snapshot_bundle(), before);
    assert!(runtime.poll_delta().is_none());
}

#[test]
fn dropped_direct_batch_emits_no_delta_but_does_not_auto_rollback_state() {
    let mut state = DocumentState::new();
    let mut ctx = ChangeCtx::new("local");

    {
        let mut batch = ctx.begin_batch().unwrap();
        state.rename(&mut batch, "Renamed").unwrap();
        let error = state.rows.delete(&mut batch, "missing").unwrap_err();
        assert_eq!(
            error,
            SyncError::StableIdNotFound {
                id: "missing".into()
            }
        );
    }

    assert_eq!(state.snapshot().0, "Renamed");
    assert_eq!(state.snapshot().1, 1);
    assert_eq!(ctx.current_seq(), 0);
    assert!(ctx.poll().is_none());
}

#[test]
fn poisoned_direct_batch_cannot_commit_after_error() {
    let mut state = DocumentState::new();
    let before = state.snapshot();
    let mut ctx = ChangeCtx::new("local");

    let commit_result = {
        let mut batch = ctx.begin_batch().unwrap();
        state.rename(&mut batch, "Renamed").unwrap();
        let error = state.rows.delete(&mut batch, "missing").unwrap_err();
        assert_eq!(
            error,
            SyncError::StableIdNotFound {
                id: "missing".into()
            }
        );
        batch.commit()
    };

    assert_eq!(commit_result.unwrap_err(), SyncError::BatchAborted);
    assert_eq!(state.snapshot().0, "Renamed");
    assert_eq!(state.snapshot().1, 1);
    assert_ne!(state.snapshot(), before);
    assert_eq!(ctx.current_seq(), 0);
    assert!(ctx.poll().is_none());
}

#[test]
fn derive_generated_snapshot_and_field_routing_work_at_runtime() {
    let mut state = DerivedDocumentState {
        id: "doc-1".into(),
        title: SyncableString::new(SyncPath::from_field("title"), "Draft"),
        revision: syncable_state::SyncableCounter::new(SyncPath::from_field("revision"), 2),
        local_cache: 99,
    };

    let snapshot = state.snapshot();
    assert_eq!(snapshot.id, "doc-1");
    assert_eq!(snapshot.title, "Draft");
    assert_eq!(snapshot.revision, 2);
    assert_eq!(DerivedDocumentState::schema().fields.len(), 3);
    assert_eq!(DerivedDocumentState::schema().fields[1].name, "headline");
    assert_eq!(syncable_state::StableId::stable_id(&state), "doc-1");

    state
        .apply_path(
            &[PathSegment::Field("headline".into())],
            &ChangeOp::String(StringOp::Set("Published".into())),
        )
        .unwrap();
    state
        .apply_path(
            &[PathSegment::Field("revision".into())],
            &ChangeOp::Counter(CounterOp::Increment(3)),
        )
        .unwrap();

    assert_eq!(state.title.value(), "Published");
    assert_eq!(state.revision.value(), 5);
    assert_eq!(state.local_cache, 99);
}

#[test]
fn derive_rebinds_local_mutation_paths_to_wire_names_when_runtime_starts() {
    let state = DerivedDocumentState {
        id: "doc-1".into(),
        title: SyncableString::new(SyncPath::from_field("title"), "Draft"),
        revision: syncable_state::SyncableCounter::new(SyncPath::from_field("revision"), 2),
        local_cache: 0,
    };
    let mut runtime = RuntimeState::new("local", state);

    let (_, committed) = runtime
        .with_batch(|state, batch| {
            state.title.set(batch, "Published")?;
            Ok(())
        })
        .unwrap();

    assert_eq!(
        committed.unwrap().changes,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("headline"),
            ChangeOp::String(StringOp::Set("Published".into())),
        )]
    );
}
