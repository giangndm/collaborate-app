use std::collections::BTreeMap;

use syncable_state::{
    ApplyChildPath, ApplyPath, ChangeEnvelope, ChangeOp, CounterOp, FieldKind, FieldSchema, ListOp,
    PathSegment, RuntimeState, SnapshotCodec, SnapshotValue, StableId, StateSchema, StringOp,
    SyncContainer, SyncError, SyncPath, SyncRuntime, SyncableCounter, SyncableState,
    SyncableString, SyncableText, SyncableVec,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RowState {
    id: String,
    title: SyncableText,
}

impl RowState {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        let t: String = title.into();
        Self {
            id: id.into(),
            title: SyncableText::from(t),
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

    fn rebind_paths(&mut self, root_path: SyncPath, tracker: Option<syncable_state::EventTracker>) {
        let mut child_root = root_path.clone().into_vec();
        child_root.push(PathSegment::Field("title".into()));
        self.title.rebind_paths(SyncPath::new(child_root), tracker);
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
                    title: SyncableText::from(title),
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
        let mut state = Self {
            title: SyncableString::from("Doc"),
            revision: SyncableCounter::from(0),
            rows: SyncableVec::from_items(vec![
                RowState::new("a", "First"),
                RowState::new("b", "Second"),
            ])
            .unwrap(),
        };
        state.rebind_paths(SyncPath::default(), None);
        state
    }

    fn rename(&mut self, title: &str) -> Result<(), SyncError> {
        self.title.set(title)?;
        self.revision.increment(1)?;
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

    fn should_rebind_root() -> bool {
        true
    }

    fn rebind_paths(&mut self, root_path: SyncPath, tracker: Option<syncable_state::EventTracker>) {
        let mut child_root = root_path.clone().into_vec();
        child_root.push(PathSegment::Field("title".into()));
        self.title
            .rebind_paths(SyncPath::new(child_root), tracker.clone());

        let mut child_root = root_path.clone().into_vec();
        child_root.push(PathSegment::Field("revision".into()));
        self.revision
            .rebind_paths(SyncPath::new(child_root), tracker.clone());

        let mut child_root = root_path.clone().into_vec();
        child_root.push(PathSegment::Field("rows".into()));
        self.rows.rebind_paths(SyncPath::new(child_root), tracker);
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
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut state = DocumentState::new();
    state.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    state.rows.delete("a").unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(
        changes,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("rows"),
            ChangeOp::List(ListOp::Delete { id: "a".into() }),
        )]
    );
}

#[test]
fn rename_emits_title_and_revision_changes() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut state = DocumentState::new();
    state.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    state.rename("Renamed").unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(changes.len(), 2);
    assert_eq!(
        changes[0],
        ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::String(StringOp::Set("Renamed".into())),
        )
    );
    assert_eq!(
        changes[1],
        ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )
    );
}

#[test]
fn snapshot_sequence_and_polled_delta_sequence_stay_aligned() {
    let mut runtime = RuntimeState::new("local", DocumentState::new());

    runtime.rename("Renamed").unwrap();
    let delta1 = runtime.poll().unwrap();

    runtime.rows.delete("a").unwrap();
    let delta2 = runtime.poll().unwrap();

    let snapshot = runtime.snapshot_bundle();
    let emitted = vec![delta1, delta2];
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

// Removed transaction isolation tests specific to the obsolete `with_batch` closure

#[test]
fn derive_generated_snapshot_and_field_routing_work_at_runtime() {
    let mut state = DerivedDocumentState {
        id: "doc-1".into(),
        title: SyncableString::from("Draft"),
        revision: syncable_state::SyncableCounter::from(2),
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
        title: SyncableString::from("Draft"),
        revision: syncable_state::SyncableCounter::from(2),
        local_cache: 0,
    };
    let mut runtime = RuntimeState::new("local", state);

    runtime.title.set("Published").unwrap();
    let committed = runtime.poll().unwrap();

    assert_eq!(
        committed.changes,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("headline"),
            ChangeOp::String(StringOp::Set("Published".into())),
        )]
    );
}
