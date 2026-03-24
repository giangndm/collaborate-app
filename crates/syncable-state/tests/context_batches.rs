use syncable_state::{ChangeCtx, ChangeEnvelope, ChangeOp, CounterOp, StringOp, SyncPath};

#[test]
fn committed_batch_advances_seq_by_one() {
    let mut ctx = ChangeCtx::new("r1");
    let mut batch = ctx.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("title"),
        ChangeOp::String(StringOp::Set("A".into())),
    ));

    let committed = batch.commit().unwrap().unwrap();
    assert_eq!(committed.from_seq, 0);
    assert_eq!(committed.to_seq, 1);
    assert_eq!(committed.changes.len(), 1);
}

#[test]
fn poll_returns_committed_batches_in_order() {
    let mut ctx = ChangeCtx::new("r1");

    let mut batch1 = ctx.begin_batch().unwrap();
    batch1.push(ChangeEnvelope::new(
        SyncPath::from_field("title"),
        ChangeOp::String(StringOp::Set("A".into())),
    ));
    batch1.commit().unwrap().unwrap();

    let mut batch2 = ctx.begin_batch().unwrap();
    batch2.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    batch2.commit().unwrap().unwrap();

    assert_eq!(ctx.poll().unwrap().to_seq, 1);
    assert_eq!(ctx.poll().unwrap().to_seq, 2);
    assert!(ctx.poll().is_none());
}

#[test]
fn empty_batch_commit_is_a_noop() {
    let mut ctx = ChangeCtx::new("r1");

    let batch = ctx.begin_batch().unwrap();
    let committed = batch.commit().unwrap();

    assert!(committed.is_none());
    assert_eq!(ctx.current_seq(), 0);
    assert!(ctx.poll().is_none());
}
