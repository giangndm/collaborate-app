use syncable_state::{
    ChangeOp, DeltaBatch, RuntimeState, SnapshotValue, SyncPath, SyncableState, SyncableText,
    TextOp,
};

#[test]
fn splice_updates_materialized_text_and_enqueues_splice_change() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut text = SyncableText::from("hello world");
    text.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    text.splice(6, 5, "friend").unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(text.value(), "hello friend");
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
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
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut text = SyncableText::from("hello");
    text.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    text.clear().unwrap();

    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(text.value(), "");
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::Text(TextOp::Clear),
        )]
    );
}

#[test]
fn snapshot_returns_plain_string() {
    let text = SyncableText::from("hello");

    assert_eq!(syncable_state::SyncableState::snapshot(&text), "hello");
}

#[test]
fn remote_splice_updates_materialized_text_in_one_step() {
    let mut runtime = RuntimeState::new("local", SyncableText::from("hello world"));

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
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
