use syncable_state::{
    ChangeCtx, ChangeOp, CounterOp, DeltaBatch, RuntimeState, SyncError, SyncPath, SyncableCounter,
};

#[test]
fn increment_updates_materialized_value_and_enqueues_counter_change_in_batch() {
    let mut counter = SyncableCounter::new(SyncPath::from_field("count"), 2);
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    counter.increment(&mut batch, 3).unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(counter.value(), 5);
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("count"),
            ChangeOp::Counter(CounterOp::Increment(3)),
        )]
    );
}

#[test]
fn decrement_updates_materialized_value_and_enqueues_counter_change_in_batch() {
    let mut counter = SyncableCounter::new(SyncPath::from_field("count"), 5);
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    counter.decrement(&mut batch, 2).unwrap();
    let committed = batch.commit().unwrap().unwrap();

    assert_eq!(counter.value(), 3);
    assert_eq!(
        committed.changes,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("count"),
            ChangeOp::Counter(CounterOp::Decrement(2)),
        )]
    );
}

#[test]
fn snapshot_returns_plain_i64() {
    let counter = SyncableCounter::new(SyncPath::from_field("count"), 7);

    assert_eq!(syncable_state::SyncableState::snapshot(&counter), 7);
}

#[test]
fn remote_apply_handles_counter_ops_deterministically_through_path_application() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableCounter::new(SyncPath::from_field("count"), 0),
    );

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::from_field("count"),
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
                SyncPath::from_field("count"),
                ChangeOp::Counter(CounterOp::Decrement(1)),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.state().value(), 3);
}

#[test]
fn stale_duplicate_remote_counter_batch_is_idempotent() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableCounter::new(SyncPath::from_field("count"), 0),
    );
    let batch = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![syncable_state::ChangeEnvelope::new(
            SyncPath::from_field("count"),
            ChangeOp::Counter(CounterOp::Increment(4)),
        )],
    );

    runtime.apply_remote(batch.clone()).unwrap();
    runtime.apply_remote(batch).unwrap();

    assert_eq!(runtime.state().value(), 4);
}

#[test]
fn local_increment_returns_overflow_error_without_mutating_state() {
    let mut counter = SyncableCounter::new(SyncPath::from_field("count"), i64::MAX);
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    let error = counter.increment(&mut batch, 1).unwrap_err();

    assert_eq!(counter.value(), i64::MAX);
    assert_eq!(batch.commit().unwrap_err(), SyncError::BatchAborted);
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
    let mut counter = SyncableCounter::new(SyncPath::from_field("count"), 3);
    let mut ctx = ChangeCtx::new("local");
    let mut batch = ctx.begin_batch().unwrap();

    let error = counter.increment(&mut batch, -1).unwrap_err();

    assert_eq!(counter.value(), 3);
    assert_eq!(batch.commit().unwrap_err(), SyncError::BatchAborted);
    assert_eq!(error, SyncError::InvalidCounterAmount);
}

#[test]
fn remote_apply_returns_overflow_error_without_mutating_state() {
    let mut runtime = RuntimeState::new(
        "local",
        SyncableCounter::new(SyncPath::from_field("count"), i64::MAX),
    );

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::from_field("count"),
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
    let mut runtime = RuntimeState::new(
        "local",
        SyncableCounter::new(SyncPath::from_field("count"), 3),
    );

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![syncable_state::ChangeEnvelope::new(
                SyncPath::from_field("count"),
                ChangeOp::Counter(CounterOp::Increment(-1)),
            )],
        ))
        .unwrap_err();

    assert_eq!(runtime.state().value(), 3);
    assert_eq!(error, SyncError::InvalidCounterAmount);
}
