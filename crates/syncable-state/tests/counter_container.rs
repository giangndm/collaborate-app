use syncable_state::{
    ChangeOp, CounterOp, DeltaBatch, RuntimeState, SyncError, SyncPath, SyncableCounter,
    SyncableState,
};

#[test]
fn increment_updates_materialized_value_and_enqueues_counter_change_in_batch() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut counter = SyncableCounter::from(2);
    counter.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    counter.increment(3).unwrap();
    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(counter.value(), 5);
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::Counter(CounterOp::Increment(3)),
        )]
    );
}

#[test]
fn decrement_updates_materialized_value_and_enqueues_counter_change_in_batch() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut counter = SyncableCounter::from(5);
    counter.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    counter.decrement(2).unwrap();
    let changes = tracker.borrow_mut().drain(..).collect::<Vec<_>>();

    assert_eq!(counter.value(), 3);
    assert_eq!(
        changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::Counter(CounterOp::Decrement(2)),
        )]
    );
}

#[test]
fn snapshot_returns_plain_i64() {
    let counter = SyncableCounter::from(7);

    assert_eq!(syncable_state::SyncableState::snapshot(&counter), 7);
}

#[test]
fn remote_apply_handles_counter_ops_deterministically_through_path_application() {
    let mut runtime = RuntimeState::new("local", SyncableCounter::from(0));

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
                ChangeOp::Counter(CounterOp::Increment(4)),
            )],
        ))
        .unwrap();
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            1,
            2,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
                ChangeOp::Counter(CounterOp::Decrement(1)),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().value(), 3);
}

#[test]
fn stale_duplicate_remote_counter_batch_is_idempotent() {
    let mut runtime = RuntimeState::new("local", SyncableCounter::from(0));
    let batch = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::default(),
            ChangeOp::Counter(CounterOp::Increment(4)),
        )],
    );

    runtime.apply_remote(batch.clone()).unwrap();
    runtime.apply_remote(batch).unwrap();

    assert_eq!(runtime.state().value(), 4);
}

#[test]
fn local_increment_returns_overflow_error_without_mutating_state() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut counter = SyncableCounter::from(i64::MAX);
    counter.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    let error = counter.increment(1).unwrap_err();

    assert_eq!(counter.value(), i64::MAX);
    assert!(tracker.borrow().is_empty());
    assert_eq!(
        error,
        SyncError::CounterOverflow {
            current: i64::MAX,
            delta: 1,
        }
    );
}

#[test]
fn local_increment_rejects_negative_amounts() {
    let tracker = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut counter = SyncableCounter::from(3);
    counter.rebind_paths(SyncPath::default(), Some(tracker.clone()));

    let error = counter.increment(-1).unwrap_err();

    assert_eq!(counter.value(), 3);
    assert!(tracker.borrow().is_empty());
    assert_eq!(error, SyncError::InvalidCounterAmount);
}

#[test]
fn remote_apply_returns_overflow_error_without_mutating_state() {
    let mut runtime = RuntimeState::new("local", SyncableCounter::from(i64::MAX));

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        ))
        .unwrap_err();

    assert_eq!(runtime.state().value(), i64::MAX);
    assert_eq!(
        error,
        SyncError::CounterOverflow {
            current: i64::MAX,
            delta: 1,
        }
    );
}

#[test]
fn remote_apply_rejects_negative_counter_amounts() {
    let mut runtime = RuntimeState::new("local", SyncableCounter::from(3));

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::default(),
                ChangeOp::Counter(CounterOp::Increment(-1)),
            )],
        ))
        .unwrap_err();

    assert_eq!(runtime.state().value(), 3);
    assert_eq!(error, SyncError::InvalidCounterAmount);
}
