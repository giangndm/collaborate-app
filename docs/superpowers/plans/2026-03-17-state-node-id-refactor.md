# State Node ID Refactor Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `State` so identity is configured through public constructors, `.doc` stays private, and `poll()` reads as explicit sync orchestration instead of mixed CRDT plumbing.

**Architecture:** Add a public `State::with_node_id(node_id, state)` constructor and keep `State::new(state)` as a convenience constructor that generates a random node id string. Move Automerge actor initialization behind those constructors, and split `poll()` into small helpers for queue draining, publish decision, and state publishing so sync bootstrap behavior remains correct but easier to reason about.

**Tech Stack:** Rust, `automerge`, `automorph`, `rand`, `cargo test`

---

## Chunk 1: Constructor API and Poll Refactor

### Task 1: Add failing tests for the public identity API

**Files:**
- Modify: `crates/collaborate-room/Cargo.toml`
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Replace all direct `.doc.set_actor(...)` test setup with the new constructor API and add a constructor test proving `State::new` still produces a usable syncing replica.

```rust
#[test_log::test]
fn test_state_sync_bootstraps_remote_state_without_conflict() {
    let mut state1 = State::with_node_id("00", TestState::default());
    let mut state2 = State::with_node_id("ff", TestState::default());

    state1.v = 42;
    state1.s = "hello".to_string();

    while let Some(change) = state1.poll() {
        state2.apply(change);
    }

    assert_eq!(state1.deref(), state2.deref());
}

#[test_log::test]
fn test_state_sync_with_random_node_id() {
    let mut state1 = State::new(TestState::default());
    let mut state2 = State::new(TestState::default());

    state1.v = 7;
    state1.s = "random".to_string();

    while let Some(change) = state1.poll() {
        state2.apply(change);
    }

    assert_eq!(state1.deref(), state2.deref());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p collaborate-room test_state_sync -- --nocapture`
Expected: FAIL because `with_node_id` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add the `rand` dependency in `crates/collaborate-room/Cargo.toml`, then add constructor helpers that own Automerge actor initialization and keep `doc` private.

```rust
impl<S: Automorph> State<S> {
    pub fn new(state: S) -> Self {
        Self::with_node_id(random_node_id(), state)
    }

    pub fn with_node_id(node_id: impl AsRef<str>, state: S) -> Self {
        Self {
            state,
            doc: doc_with_node_id(node_id.as_ref()),
            changes: VecDeque::new(),
        }
    }
}

fn random_node_id() -> String { ... }
fn doc_with_node_id(node_id: &str) -> AutoCommit { ... }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p collaborate-room test_state_sync -- --nocapture`
Expected: PASS

### Task 2: Refactor `poll()` into clear helpers

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Write the failing test if behavior changes during refactor**

If needed, add one focused test for the bootstrap path so helper extraction preserves behavior.

- [ ] **Step 2: Run targeted test to lock behavior**

Run: `cargo test -p collaborate-room tests::test_state_sync_bootstraps_remote_state_without_conflict -- --exact --nocapture`
Expected: PASS before refactor.

- [ ] **Step 3: Write minimal refactor**

Extract helper methods so `poll()` reads like orchestration.

```rust
pub fn poll(&mut self) -> Option<Change> {
    if let Some(change) = self.pop_pending_change() {
        return Some(change);
    }

    self.publish_if_needed()?;
    self.pop_pending_change()
}

fn pop_pending_change(&mut self) -> Option<Change> { ... }
fn publish_if_needed(&mut self) -> Option<()> { ... }
fn needs_publish(&self) -> bool { ... }
fn state_exists(&self) -> bool { ... }
fn save_state(&mut self) -> Option<()> { ... }
fn queue_all_changes(&mut self) { ... }
```

Keep external behavior unchanged: missing state should publish once, existing synced state should not publish when `diff.none()`, and save failures should log and return `None`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p collaborate-room test_state_sync -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run repetition check**

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
