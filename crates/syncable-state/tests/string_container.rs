use syncable_state::{
    ApplyPath, ChangeCtx, ChangeOp, DeltaBatch, PathSegment, RuntimeState, StateSchema, StringOp,
    SyncContainer, SyncError, SyncPath, SyncableCounter, SyncableState, SyncableString,
};

#[test]
fn set_updates_local_value_and_enqueues_string_set_change_in_batch() {
    let mut value = SyncableString::new(SyncPath::from_field("title"), "before");
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    value.set(&mut batch, "after").unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(value.value(), "after");
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::String(StringOp::Set("after".into())),
        )]
    );
}

#[test]
fn clear_enqueues_clear_change_in_batch() {
    let mut value = SyncableString::new(SyncPath::from_field("title"), "filled");
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    value.clear(&mut batch).unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(value.value(), "");
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::String(StringOp::Clear),
        )]
    );
}

#[test]
fn snapshot_returns_plain_string() {
    let value = SyncableString::new(SyncPath::from_field("title"), "hello");

    assert_eq!(
        syncable_state::SyncableState::snapshot(&value),
        "hello".to_string()
    );
}

#[test]
fn remote_apply_on_root_field_path_updates_materialized_state() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableString::new(SyncPath::from_field("title"), "before"),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::from_field("title"),
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
        title: SyncableString::new(SyncPath::from_field("title"), "before"),
        revision: SyncableCounter::new(SyncPath::from_field("revision"), 0),
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
    let mut value = SyncableString::new(SyncPath::from_field("title"), "before");

    let error = value
        .apply_path(&[], &ChangeOp::String(StringOp::Set("after".into())))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidPath);
}
