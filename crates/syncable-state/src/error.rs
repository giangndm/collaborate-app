use crate::ReplicaId;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SyncError {
    #[error(
        "invalid batch sequence for replica {replica_id}: expected a single-step batch, got {from_seq}->{to_seq}"
    )]
    InvalidBatchSequence {
        replica_id: ReplicaId,
        from_seq: u64,
        to_seq: u64,
    },
    #[error("empty batch is not allowed for replica {replica_id}")]
    EmptyBatch { replica_id: ReplicaId },
    #[error(
        "gap detected for replica {replica_id}: expected from_seq {expected_from}, got {actual_from}"
    )]
    GapDetected {
        replica_id: ReplicaId,
        expected_from: u64,
        actual_from: u64,
    },
    #[error(
        "stale batch for replica {replica_id}: local seq {local_seq}, batch {from_seq}->{to_seq}"
    )]
    StaleBatch {
        replica_id: ReplicaId,
        local_seq: u64,
        from_seq: u64,
        to_seq: u64,
    },
    #[error(
        "conflicting replay for replica {replica_id} at to_seq {to_seq}: batch key was already seen with different contents"
    )]
    ConflictingReplay { replica_id: ReplicaId, to_seq: u64 },
    #[error("authoritative replica mismatch: expected {expected}, got {actual}")]
    AuthorityMismatch {
        expected: ReplicaId,
        actual: ReplicaId,
    },
    #[error(
        "authoritative role conflict: local replica {local_replica_id} already established an outbound stream before remote replica {remote_replica_id} applied"
    )]
    RoleConflict {
        local_replica_id: ReplicaId,
        remote_replica_id: ReplicaId,
    },
    #[error(
        "snapshot/bootstrap mismatch: snapshot stream {snapshot_replica_id}@{snapshot_seq} does not match bootstrap stream {bootstrap_replica_id}@{bootstrap_seq}"
    )]
    BootstrapMismatch {
        snapshot_replica_id: ReplicaId,
        snapshot_seq: u64,
        bootstrap_replica_id: ReplicaId,
        bootstrap_seq: u64,
    },
    #[error("invalid bootstrap state: {reason}")]
    InvalidBootstrap { reason: String },
    #[error("restored state does not match the supplied snapshot payload")]
    SnapshotStateMismatch,
    #[error("counter amount must be non-negative")]
    InvalidCounterAmount,
    #[error("counter overflow for current value {current} with delta {delta}")]
    CounterOverflow { current: i64, delta: i64 },
    #[error("invalid text splice range")]
    InvalidTextRange,
    #[error("invalid snapshot value")]
    InvalidSnapshotValue,
    #[error("stable id already exists: {id}")]
    DuplicateStableId { id: String },
    #[error("duplicate map key: {key}")]
    DuplicateMapKey { key: String },
    #[error("stable id not found: {id}")]
    StableIdNotFound { id: String },
    #[error("batch was aborted due to a prior mutation error")]
    BatchAborted,
    #[error("invalid path")]
    InvalidPath,
}
