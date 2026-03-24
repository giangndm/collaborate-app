use syncable_state::{
    ApplyPath, ChangeCtx, ChangeEnvelope, ChangeOp, CounterOp, DeltaBatch, PathSegment,
    RuntimeState, SnapshotBundle, StateSchema, StringOp, SyncError, SyncPath, SyncableState,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TestDoc {
    title: String,
    revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct FragileDoc {
    title: String,
    reject_revision: bool,
}

impl ApplyPath for TestDoc {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([PathSegment::Field(field)], ChangeOp::String(StringOp::Set(value)))
                if field == "title" =>
            {
                self.title = value.clone();
                Ok(())
            }
            ([PathSegment::Field(field)], ChangeOp::Counter(CounterOp::Increment(by)))
                if field == "revision" =>
            {
                self.revision += by;
                Ok(())
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for TestDoc {
    type Snapshot = Self;

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn schema() -> StateSchema {
        StateSchema::default()
    }
}

impl ApplyPath for FragileDoc {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([PathSegment::Field(field)], ChangeOp::String(StringOp::Set(value)))
                if field == "title" =>
            {
                self.title = value.clone();
                Ok(())
            }
            ([PathSegment::Field(field)], ChangeOp::Counter(CounterOp::Increment(_)))
                if field == "revision" && self.reject_revision =>
            {
                Err(SyncError::InvalidPath)
            }
            ([PathSegment::Field(field)], ChangeOp::Counter(CounterOp::Increment(_)))
                if field == "revision" =>
            {
                Ok(())
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl SyncableState for FragileDoc {
    type Snapshot = Self;

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn schema() -> StateSchema {
        StateSchema::default()
    }
}

#[test]
fn apply_remote_batch_advances_state_when_sequences_are_contiguous() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("Hello".into())),
            )],
        ))
        .unwrap();

    assert_eq!(runtime.current_seq(), 1);
    assert_eq!(runtime.state().title, "Hello");
    assert_eq!(
        runtime.snapshot(),
        SnapshotBundle {
            replica_id: "remote".into(),
            seq: 1,
            snapshot: TestDoc {
                title: "Hello".into(),
                revision: 0,
            },
        }
    );
}

#[test]
fn apply_remote_detects_sequence_gaps() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            2,
            3,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::GapDetected {
            replica_id: "remote".into(),
            expected_from: 0,
            actual_from: 2,
        }
    );
}

#[test]
fn apply_remote_ignores_stale_duplicate_replay_idempotently() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    let batch = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(2)),
        )],
    );

    runtime.apply_remote(batch.clone()).unwrap();
    runtime.apply_remote(batch).unwrap();

    assert_eq!(runtime.current_seq(), 1);
    assert_eq!(runtime.state().revision, 2);
}

#[test]
fn conflicting_replay_with_same_batch_key_is_rejected() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    let original = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(2)),
        )],
    );

    runtime.apply_remote(original).unwrap();

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(3)),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::ConflictingReplay {
            replica_id: "remote".into(),
            to_seq: 1,
        }
    );
}

#[test]
fn remote_batch_is_rejected_when_local_stream_is_already_authoritative() {
    let mut ctx = ChangeCtx::new("local");
    let mut local_batch = ctx.begin_batch().unwrap();
    local_batch.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    local_batch.commit().unwrap().unwrap();

    let mut doc = TestDoc::default();
    let error = ctx
        .apply_remote(
            &mut doc,
            DeltaBatch::new(
                "remote",
                0,
                1,
                vec![ChangeEnvelope::new(
                    SyncPath::from_field("title"),
                    ChangeOp::String(StringOp::Set("ignored".into())),
                )],
            ),
        )
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::RoleConflict {
            local_replica_id: "local".into(),
            remote_replica_id: "remote".into(),
        }
    );
}

#[test]
fn local_batch_is_rejected_after_remote_authority_is_established() {
    let mut ctx = ChangeCtx::new("local");
    let mut doc = TestDoc::default();

    ctx.apply_remote(
        &mut doc,
        DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("remote".into())),
            )],
        ),
    )
    .unwrap();

    let error = match ctx.begin_batch() {
        Ok(_) => panic!("expected begin_batch to reject local writes after remote authority"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        SyncError::RoleConflict {
            local_replica_id: "local".into(),
            remote_replica_id: "remote".into(),
        }
    );
}

#[test]
fn established_remote_authority_rejects_wrong_replica() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    runtime
        .apply_remote(DeltaBatch::new(
            "remote-a",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("hello".into())),
            )],
        ))
        .unwrap();

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote-b",
            1,
            2,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::AuthorityMismatch {
            expected: "remote-a".into(),
            actual: "remote-b".into(),
        }
    );
}

#[test]
fn malformed_remote_batch_with_non_unit_step_is_rejected() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            2,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("Hello".into())),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBatchSequence {
            replica_id: "remote".into(),
            from_seq: 0,
            to_seq: 2,
        }
    );
}

#[test]
fn malformed_remote_batch_with_non_advancing_seq_is_rejected() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            1,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("Hello".into())),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBatchSequence {
            replica_id: "remote".into(),
            from_seq: 1,
            to_seq: 1,
        }
    );
}

#[test]
fn remote_apply_is_atomic_when_a_later_change_fails() {
    let mut runtime = RuntimeState::new(
        "local",
        FragileDoc {
            title: "before".into(),
            reject_revision: true,
        },
    );

    let error = runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![
                ChangeEnvelope::new(
                    SyncPath::from_field("title"),
                    ChangeOp::String(StringOp::Set("after".into())),
                ),
                ChangeEnvelope::new(
                    SyncPath::from_field("revision"),
                    ChangeOp::Counter(CounterOp::Increment(1)),
                ),
            ],
        ))
        .unwrap_err();

    assert_eq!(error, SyncError::InvalidPath);
    assert_eq!(runtime.current_seq(), 0);
    assert_eq!(runtime.state().title, "before");
    assert!(runtime.poll().is_none());
}

#[test]
fn remote_apply_does_not_requeue_outbound_deltas() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("Hello".into())),
            )],
        ))
        .unwrap();

    assert!(runtime.poll().is_none());
}

#[test]
fn same_to_seq_from_different_replica_is_not_treated_as_a_duplicate() {
    let mut ctx = ChangeCtx::new("local");
    let first = DeltaBatch::new(
        "remote-a",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    );
    let second = DeltaBatch::new(
        "remote-b",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    );
    let mut doc = TestDoc::default();

    ctx.apply_remote(&mut doc, first).unwrap();
    let error = ctx.apply_remote(&mut doc, second).unwrap_err();

    assert_eq!(
        error,
        SyncError::AuthorityMismatch {
            expected: "remote-a".into(),
            actual: "remote-b".into(),
        }
    );
}

#[test]
fn old_seen_batch_is_still_ignored_idempotently_later() {
    let mut ctx = ChangeCtx::new("local");
    let mut doc = TestDoc::default();
    let retained = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    );

    ctx.apply_remote(&mut doc, retained.clone()).unwrap();

    for seq in 1..=64 {
        ctx.apply_remote(
            &mut doc,
            DeltaBatch::new(
                "remote",
                seq,
                seq + 1,
                vec![ChangeEnvelope::new(
                    SyncPath::from_field("revision"),
                    ChangeOp::Counter(CounterOp::Increment(1)),
                )],
            ),
        )
        .unwrap();
    }

    ctx.apply_remote(&mut doc, retained).unwrap();

    assert_eq!(ctx.current_seq(), 65);
}

#[test]
fn conflicting_old_stale_batch_is_rejected_even_long_after_first_seen() {
    let mut ctx = ChangeCtx::new("local");
    let mut doc = TestDoc::default();
    let original = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    );

    ctx.apply_remote(&mut doc, original).unwrap();

    for seq in 1..=64 {
        ctx.apply_remote(
            &mut doc,
            DeltaBatch::new(
                "remote",
                seq,
                seq + 1,
                vec![ChangeEnvelope::new(
                    SyncPath::from_field("revision"),
                    ChangeOp::Counter(CounterOp::Increment(1)),
                )],
            ),
        )
        .unwrap();
    }

    let error = ctx
        .apply_remote(
            &mut doc,
            DeltaBatch::new(
                "remote",
                0,
                1,
                vec![ChangeEnvelope::new(
                    SyncPath::from_field("revision"),
                    ChangeOp::Counter(CounterOp::Increment(2)),
                )],
            ),
        )
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::ConflictingReplay {
            replica_id: "remote".into(),
            to_seq: 1,
        }
    );
}

#[test]
fn restored_runtime_continues_remote_stream_and_preserves_duplicate_proof() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    let first = DeltaBatch::new(
        "remote",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("title"),
            ChangeOp::String(StringOp::Set("restored".into())),
        )],
    );

    runtime.apply_remote(first.clone()).unwrap();

    let snapshot = runtime.snapshot();
    let restored_meta = runtime.bootstrap();
    let mut restored = RuntimeState::restore(
        "local",
        snapshot.snapshot.clone(),
        snapshot.clone(),
        restored_meta,
    )
    .unwrap();

    restored
        .apply_remote(DeltaBatch::new(
            "remote",
            1,
            2,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        ))
        .unwrap();

    restored.apply_remote(first).unwrap();

    assert_eq!(restored.current_seq(), 2);
    assert_eq!(restored.state().title, "restored");
    assert_eq!(restored.state().revision, 1);
}

#[test]
fn restored_runtime_rejects_conflicting_old_replay_from_persisted_metadata() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        ))
        .unwrap();

    let snapshot = runtime.snapshot();
    let restored_meta = runtime.bootstrap();
    let mut restored =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, restored_meta).unwrap();

    let error = restored
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(9)),
            )],
        ))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::ConflictingReplay {
            replica_id: "remote".into(),
            to_seq: 1,
        }
    );
}

#[test]
fn restore_rejects_mismatched_snapshot_and_bootstrap_metadata() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("restored".into())),
            )],
        ))
        .unwrap();

    let mut bootstrap = runtime.bootstrap();
    let snapshot = runtime.snapshot();
    bootstrap.seq = 2;

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::BootstrapMismatch {
            snapshot_replica_id: "remote".into(),
            snapshot_seq: 1,
            bootstrap_replica_id: "remote".into(),
            bootstrap_seq: 2,
        }
    );
}

#[test]
fn restore_rejects_state_that_does_not_match_snapshot_payload() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("restored".into())),
            )],
        ))
        .unwrap();

    let snapshot = runtime.snapshot();
    let bootstrap = runtime.bootstrap();

    let error = RuntimeState::restore(
        "local",
        TestDoc {
            title: "corrupt".into(),
            revision: 0,
        },
        snapshot,
        bootstrap,
    )
    .unwrap_err();

    assert_eq!(error, SyncError::SnapshotStateMismatch);
}

#[test]
fn empty_remote_batch_is_rejected() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());

    let error = runtime
        .apply_remote(DeltaBatch::new("remote", 0, 1, vec![]))
        .unwrap_err();

    assert_eq!(
        error,
        SyncError::EmptyBatch {
            replica_id: "remote".into(),
        }
    );
}

#[test]
fn restore_preserves_pending_local_batches() {
    let mut runtime = RuntimeState::new(
        "local",
        TestDoc {
            title: "local".into(),
            revision: 0,
        },
    );

    let mut batch = runtime.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    let committed = batch.commit().unwrap().unwrap();

    let snapshot = runtime.snapshot();
    let bootstrap = runtime.bootstrap();

    let mut restored =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap();

    assert_eq!(restored.poll(), Some(committed));
    assert!(restored.poll().is_none());
}

#[test]
fn restore_rejects_bootstrap_with_mismatched_remote_authority() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("restored".into())),
            )],
        ))
        .unwrap();

    let snapshot = runtime.snapshot();
    let mut bootstrap = runtime.bootstrap();
    bootstrap.remote_authority = Some("other-remote".into());

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBootstrap {
            reason: "remote_authority must match stream_replica_id".into(),
        }
    );
}

#[test]
fn restore_rejects_bootstrap_with_pending_batches_under_remote_authority() {
    let mut runtime = RuntimeState::new("local", TestDoc::default());
    runtime
        .apply_remote(DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("title"),
                ChangeOp::String(StringOp::Set("restored".into())),
            )],
        ))
        .unwrap();

    let snapshot = runtime.snapshot();
    let mut bootstrap = runtime.bootstrap();
    bootstrap.pending.push(DeltaBatch::new(
        "local",
        1,
        2,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    ));

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBootstrap {
            reason: "pending local batches cannot coexist with remote authority".into(),
        }
    );
}

#[test]
fn restore_rejects_bootstrap_with_non_contiguous_pending_batches() {
    let mut runtime = RuntimeState::new(
        "local",
        TestDoc {
            title: "local".into(),
            revision: 0,
        },
    );

    let mut batch = runtime.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    let _ = batch.commit();

    let snapshot = runtime.snapshot();
    let mut bootstrap = runtime.bootstrap();
    bootstrap.pending.push(DeltaBatch::new(
        "local",
        0,
        1,
        vec![ChangeEnvelope::new(
            SyncPath::from_field("revision"),
            ChangeOp::Counter(CounterOp::Increment(1)),
        )],
    ));

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBootstrap {
            reason: "pending batches must form one contiguous sequence".into(),
        }
    );
}

#[test]
fn restore_rejects_local_authority_bootstrap_with_foreign_seen_batch() {
    let mut runtime = RuntimeState::new(
        "local",
        TestDoc {
            title: "local".into(),
            revision: 0,
        },
    );

    let mut batch = runtime.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    let _ = batch.commit();

    let snapshot = runtime.snapshot();
    let mut bootstrap = runtime.bootstrap();
    bootstrap.seen_batches.push(
        DeltaBatch::new(
            "remote",
            0,
            1,
            vec![ChangeEnvelope::new(
                SyncPath::from_field("revision"),
                ChangeOp::Counter(CounterOp::Increment(1)),
            )],
        )
        .proof(),
    );

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBootstrap {
            reason: "seen batch replica must match local replica under local authority".into(),
        }
    );
}

#[test]
fn restore_rejects_bootstrap_with_duplicate_seen_batch_keys() {
    let mut runtime = RuntimeState::new(
        "local",
        TestDoc {
            title: "local".into(),
            revision: 0,
        },
    );

    let mut batch = runtime.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    let _ = batch.commit();

    let snapshot = runtime.snapshot();
    let mut bootstrap = runtime.bootstrap();
    let duplicate = bootstrap.seen_batches[0].clone();
    bootstrap.seen_batches.push(duplicate);

    let error =
        RuntimeState::restore("local", snapshot.snapshot.clone(), snapshot, bootstrap).unwrap_err();

    assert_eq!(
        error,
        SyncError::InvalidBootstrap {
            reason: "seen batch proofs cannot contain duplicate keys".into(),
        }
    );
}
