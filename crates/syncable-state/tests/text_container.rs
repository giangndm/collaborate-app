use syncable_state::{
    ChangeCtx, ChangeOp, DeltaBatch, RuntimeState, SnapshotValue, SyncPath, SyncableText, TextOp,
};

#[test]
fn splice_updates_materialized_text_and_enqueues_splice_change() {
    let mut text = SyncableText::new(SyncPath::from_field("title"), "hello world");
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    text.splice(&mut batch, 6, 5, "friend").unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(text.value(), "hello friend");
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::Text(TextOp::Splice {
                index: 6,
                delete: 5,
                insert: "friend".into(),
            }),
        )]
    );
}

#[test]
fn clear_enqueues_clear_change_and_clears_materialized_text() {
    let mut text = SyncableText::new(SyncPath::from_field("title"), "hello");
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    text.clear(&mut batch).unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(text.value(), "");
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::Text(TextOp::Clear),
        )]
    );
}

#[test]
fn snapshot_returns_plain_string() {
    let text = SyncableText::new(SyncPath::from_field("title"), "hello");

    assert_eq!(syncable_state::SyncableState::snapshot(&text), "hello");
}

#[test]
fn remote_splice_updates_materialized_text_in_one_step() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableText::new(SyncPath::from_field("title"), "hello world"),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::Text(TextOp::Splice {
                    index: 0,
                    delete: 5,
                    insert: "hi".into(),
                }),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().value(), "hi world");
    assert_eq!(runtime.snapshot().snapshot, "hi world".to_string());
    assert_ne!(
        SnapshotValue::String(runtime.snapshot().snapshot),
        SnapshotValue::Null
    );
}
