# State Incremental Sync Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `State::poll()` export only unsent Automerge changes instead of the full document history so repeated sync throughput scales much better.

**Architecture:** Add sender-side tracking for the last exported Automerge heads inside `State<S>`, then have `poll()` queue only `doc.get_changes(&last_sent_heads)` after a successful save. Keep the public API unchanged, preserve current correctness tests, and extend tests/benchmarks so they prove both sync correctness and the intended performance-sensitive behavior.

**Tech Stack:** Rust, Automerge, Automorph, Criterion, `cargo test`, `cargo bench`

---

## Chunk 1: Lock in the desired sync behavior

### Task 1: Add a regression test for incremental export

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add a unit test that proves a second local mutation emits only the newly created outbound change set instead of replaying all earlier history. Compare the emitted batch against an explicit saved frontier, not just final state.

```rust
#[test_log::test]
fn test_poll_only_emits_unsent_changes() {
    let mut sender = State::with_node_id("sender", TestState::default());

    sender.v = 1;
    let first_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
    assert!(!first_batch.is_empty());
    let frontier_after_first_publish = sender.doc.get_heads();

    sender.v = 2;
    let second_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
    assert!(!second_batch.is_empty());
    let expected_delta = sender.doc.get_changes(&frontier_after_first_publish);

    assert_eq!(second_batch.len(), expected_delta.len());
    assert_eq!(second_batch, expected_delta);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p collaborate-room tests::test_poll_only_emits_unsent_changes -- --exact --nocapture`
Expected: FAIL because the second batch currently includes the entire document history.

- [ ] **Step 3: Write the minimal implementation**

Add a field such as `sent_heads: Vec<ChangeHash>` or the Automerge heads type already returned by `get_heads()`, initialize it in constructors, and update queueing logic to use it.

```rust
pub struct State<S> {
    state: S,
    doc: AutoCommit,
    changes: VecDeque<Change>,
    sent_heads: Vec<ChangeHash>,
}

fn queue_new_changes(&mut self) {
    self.changes.extend(self.doc.get_changes(&self.sent_heads));
    self.sent_heads = self.doc.get_heads();
}
```

Use this incremental queueing helper in place of `get_changes(&[])`.

Define the outbound contract explicitly: after `apply()` imports remote history, the next `poll()` should export only new local changes created after that import, not replay the imported remote history.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p collaborate-room tests::test_poll_only_emits_unsent_changes -- --exact --nocapture`
Expected: PASS

### Task 2: Lock the inbound-apply contract

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Write the failing inbound-contract test**

Add a test that imports a remote change into an intermediate peer, then performs one local mutation and verifies `poll()` exports only the new local delta rather than replaying imported remote history.

```rust
#[test_log::test]
fn test_poll_does_not_replay_remote_applied_history() {
    let mut original = State::with_node_id("original", TestState::default());
    let mut relay = State::with_node_id("relay", TestState::default());

    original.v = 1;
    for change in std::iter::from_fn(|| original.poll()) {
        relay.apply(change);
    }
    let frontier_after_import = relay.doc.get_heads();

    relay.s = "local".to_string();
    let forwarded: Vec<_> = std::iter::from_fn(|| relay.poll()).collect();
    let expected_delta = relay.doc.get_changes(&frontier_after_import);

    assert_eq!(forwarded.len(), expected_delta.len());
    assert_eq!(forwarded, expected_delta);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p collaborate-room tests::test_poll_does_not_replay_remote_applied_history -- --exact --nocapture`
Expected: FAIL because imported remote history is currently eligible to be re-exported on the next local publish.

- [ ] **Step 3: Update implementation minimally**

When `apply()` imports remote changes, advance the exported frontier to the current document heads so future `poll()` calls only consider newly created local history.

If needed, expose a small crate-private helper for tests to read the current document heads or expected delta without making `.doc` public. Keep that helper unavailable to normal callers.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p collaborate-room tests::test_poll_does_not_replay_remote_applied_history -- --exact --nocapture`
Expected: PASS

## Chunk 2: Preserve sync correctness

### Task 3: Re-run existing sync tests against the incremental queueing change

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Run the existing deterministic sync tests**

Run: `cargo test -p collaborate-room test_state_sync -- --nocapture`
Expected: PASS for all existing sync tests.

- [ ] **Step 2: Repeat the flaky-path verification**

Run: `python3 - <<'PY'
import subprocess, sys
for i in range(1, 21):
    r = subprocess.run(['cargo', 'test', '-q', '-p', 'collaborate-room', 'test_state_sync'])
    if r.returncode != 0:
        print(f'FAILED ON RUN {i}')
        sys.exit(r.returncode)
print('ALL PASSED')
PY`
Expected: `ALL PASSED`

## Chunk 3: Verify the benchmark improvement

### Task 4: Measure the benchmark before and after the fix

**Files:**
- Modify: `crates/collaborate-room/benches/state_sync.rs`
- Test: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Build the benchmark target**

Run the benchmark before implementing the incremental export fix and save the key numbers for `100`, `300`, and `1000` changes.

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: baseline numbers recorded for later comparison.

- [ ] **Step 2: Build the benchmark target after the fix**

Run: `cargo bench -p collaborate-room --bench state_sync --no-run`
Expected: PASS

- [ ] **Step 3: Run the benchmark and capture the new scaling shape**

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: benchmark still passes correctness assertions, and throughput degradation between `100`, `300`, and `1000` changes is much less severe than in the saved baseline.

- [ ] **Step 4: Record the result in notes or commit message context**

Summarize the before/after behavior, especially whether the benchmark trend moved from near-quadratic toward closer-to-linear growth.
