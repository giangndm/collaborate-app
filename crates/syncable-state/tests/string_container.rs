use syncable_state::{
    ApplyPath, ChangeOp, DeltaBatch, PathSegment, RuntimeState, StateSchema, StringOp,
    SyncContainer, SyncError, SyncPath, SyncableCounter, SyncableState, SyncableString,
};

#[test]
fn set_updates_local_value_and_enqueues_string_set_change_in_batch() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut value = SyncableString::from("before");
    value.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    value.set("after").unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(value.value(), "after");
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::String(StringOp::Set("after".into())),
        )]
    );
}

#[test]
fn clear_enqueues_clear_change_in_batch() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut value = SyncableString::from("filled");
    value.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    value.clear().unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(value.value(), "");
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::String(StringOp::Clear),
        )]
    );
}

#[test]
fn snapshot_returns_plain_string() {
    let value = SyncableString::from("hello");

    assert_eq!(
        syncable_state::SyncableState::snapshot(&value),
        "hello".to_string()
    );
}

#[test]
fn remote_apply_on_root_field_path_updates_materialized_state() {
    let mut runtime = RuntimeState::new("local", SyncableString::from("before"));

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
                ChangeOp::String(StringOp::Set("after".into())),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().value(), "after");
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmbeddedStringDoc {
    title: SyncableString,
    revision: SyncableCounter,
}

impl ApplyPath for EmbeddedStringDoc {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path {
            [PathSegment::Field(field), tail @ ..] if field == "title" => {
                self.title.apply_path_tail(tail, op)
            }
            [PathSegment::Field(field), tail @ ..] if field == "revision" => {
                self.revision.apply_path_tail(tail, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for EmbeddedStringDoc {
    type Snapshot = Self;

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn schema() -> StateSchema {
        StateSchema::default()
    }
}

#[test]
fn apply_path_tail_supports_parent_field_routing() {
    let mut doc = EmbeddedStringDoc {
        title: SyncableString::from("before"),
        revision: SyncableCounter::from(0),
    };

    doc.apply_path(
        &[PathSegment::Field("title".into())],
        &ChangeOp::String(StringOp::Set("after".into())),
    )
    .unwrap();

    assert_eq!(doc.title.value(), "after");
}

#[test]
fn root_apply_rejects_empty_path_without_field_identity() {
    let mut value = SyncableString::from("before");
    value.rebind_paths(SyncPath::from_field("title"), None);

    let error = value
        .apply_path(&[], &ChangeOp::String(StringOp::Set("after".into())))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidPath);
}
