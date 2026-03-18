# State Sync Bootstrap Fix Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `tests::test_state_sync` deterministic by avoiding conflicting initial Automerge bootstrap state between independently created `State` replicas.

**Architecture:** Stop writing an initial `state` object during `State::new()`. Treat a missing `state` entry as unsynced local state in `poll()`, save it on first publish, and keep `apply()` as the path that materializes remote state. This preserves each replica's independent actor history while ensuring the first outbound change establishes shared document state instead of conflicting with a private bootstrap object.

**Tech Stack:** Rust, `automerge`, `automorph`, `cargo test`

---

## Chunk 1: Regression Test and Minimal Fix

### Task 1: Add a deterministic failing regression test

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add a unit test that creates two default `State<TestState>` values, forces explicit actor IDs on the internal docs, mutates only the first one, drains all changes from `state1.poll()`, applies them to `state2`, and asserts that `state2` matches the mutated state.

```rust
#[test_log::test]
fn test_state_sync_bootstraps_remote_state_without_conflict() {
    let mut state1 = State::new(TestState::default());
    let mut state2 = State::new(TestState::default());

    state1.doc.set_actor(ActorId::try_from("00").unwrap());
    state2.doc.set_actor(ActorId::try_from("ff").unwrap());

    state1.v = 42;
    state1.s = "hello".to_string();

    while let Some(change) = state1.poll() {
        state2.apply(change);
    }

    assert_eq!(state2.deref(), state1.deref());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p collaborate-room test_state_sync_bootstraps_remote_state_without_conflict -- --exact --nocapture`
Expected: FAIL because the receiving replica can keep its own default root `state` object.

- [ ] **Step 3: Write minimal implementation**

Update `State::new()` and `State::poll()` so a new `State` starts with an empty Automerge document and only publishes a `state` map when `poll()` sees either a missing `state` entry or a real diff.

```rust
pub fn new(state: S) -> Self {
    Self {
        state,
        doc: AutoCommit::new(),
        changes: VecDeque::new(),
    }
}

pub fn poll(&mut self) -> Option<Change> {
    if let Some(change) = self.changes.pop_front() {
        return Some(change);
    }

    let state_missing = self.doc.get(ROOT, "state").ok().flatten().is_none();

    if !state_missing {
        let has_diff = self.state.diff(&self.doc, ROOT, "state").ok()?.any();
        if !has_diff {
            return None;
        }
    }

    self.state.save(&mut self.doc, ROOT, "state").ok()?;
    self.changes.extend(self.doc.get_changes(&[]));
    self.changes.pop_front()
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p collaborate-room test_state_sync_bootstraps_remote_state_without_conflict -- --exact --nocapture`
Expected: PASS

- [ ] **Step 5: Run repetition check**

Run: `python3 - <<'PY'
import subprocess, sys
for i in range(1, 21):
    print(f'RUN {i}', flush=True)
    r = subprocess.run([
        'cargo', 'test', '-p', 'collaborate-room', 'test_state_sync', '--', '--nocapture'
    ])
    if r.returncode:
        sys.exit(r.returncode)
print('ALL PASSED')
PY`
Expected: all runs pass
