# State Sync Benchmark Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a repeatable end-to-end benchmark for `collaborate-room` that measures syncing many repeated single-field state changes through the public `State` API.

**Architecture:** Add Criterion as the crate's benchmark harness and define a dedicated bench target in `crates/collaborate-room/benches/state_sync.rs`. Each benchmark iteration creates fresh sender and receiver `State` values, mutates one field repeatedly, drains `poll()` into `apply()`, and asserts convergence so the benchmark measures a real end-to-end sync path rather than isolated internals.

**Tech Stack:** Rust, Criterion, `cargo bench`, `automerge`, `automorph`

---

## Chunk 1: Benchmark Harness Setup

### Task 1: Add Criterion and bench target configuration

**Files:**
- Modify: `crates/collaborate-room/Cargo.toml`

- [ ] **Step 1: Write the failing setup expectation**

Define the intended benchmark command and target name before changing the manifest.

Run: `cargo bench -p collaborate-room --bench state_sync --no-run`
Expected: FAIL because no `state_sync` bench target exists yet.

- [ ] **Step 2: Add the minimal manifest changes**

Update `crates/collaborate-room/Cargo.toml` with Criterion as a dev-dependency and an explicit bench target.

```toml
[dev-dependencies]
test-log = { workspace = true }
criterion = "0.5"

[[bench]]
name = "state_sync"
harness = false
```

- [ ] **Step 3: Run the setup command again**

Run: `cargo bench -p collaborate-room --bench state_sync --no-run`
Expected: FAIL later because the benchmark file does not exist yet, proving the manifest wiring is active.

## Chunk 2: End-to-End Sync Benchmark

### Task 2: Add the benchmark file with one repeated-mutation scenario

**Files:**
- Create: `crates/collaborate-room/benches/state_sync.rs`
- Modify: `crates/collaborate-room/Cargo.toml`

- [ ] **Step 1: Write the benchmark code skeleton**

Create a benchmark-only state type and an end-to-end sync helper that use only the public `State` API.

```rust
use collaborate_room::State;
use automorph::Automorph;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

#[derive(Debug, Default, Automorph, PartialEq, Eq)]
struct BenchState {
    label: String,
    v: u32,
}

fn sync_all(sender: &mut State<BenchState>, receiver: &mut State<BenchState>) {
    while let Some(change) = sender.poll() {
        receiver.apply(change);
    }
}
```

- [ ] **Step 2: Write the benchmark cases**

Add a `single_field_sync` benchmark group with sizes `100`, `1_000`, and `10_000`. Inside each iteration:

1. create fresh `State::with_node_id("sender", ...)` and `State::with_node_id("receiver", ...)`
2. run `N` increments like `sender.v += 1`
3. call `sync_all(...)` after each mutation
4. verify convergence with `assert_eq!(&*sender, &*receiver)`

```rust
fn bench_single_field_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_field_sync");

    for changes in [100_u32, 1_000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(changes), &changes, |b, &changes| {
            b.iter(|| {
                let mut sender = State::with_node_id("sender", BenchState::default());
                let mut receiver = State::with_node_id("receiver", BenchState::default());

                for _ in 0..changes {
                    sender.v += 1;
                    sync_all(&mut sender, &mut receiver);
                }

                assert_eq!(&*sender, &*receiver);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_single_field_sync);
criterion_main!(benches);
```

- [ ] **Step 3: Run the benchmark target build**

Run: `cargo bench -p collaborate-room --bench state_sync --no-run`
Expected: PASS

- [ ] **Step 4: Run the benchmark**

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: Criterion output for `single_field_sync/100`, `single_field_sync/1_000`, and `single_field_sync/10_000`

## Chunk 3: Regression Safety

### Task 3: Confirm the benchmark setup did not break tests

**Files:**
- Modify: `crates/collaborate-room/Cargo.toml`
- Create: `crates/collaborate-room/benches/state_sync.rs`
- Test: `crates/collaborate-room/src/lib.rs`

- [ ] **Step 1: Run targeted sync tests**

Run: `cargo test -p collaborate-room test_state_sync -- --nocapture`
Expected: PASS

- [ ] **Step 2: Run the deterministic bootstrap sync test**

Run: `cargo test -p collaborate-room tests::test_state_sync_bootstraps_remote_state_without_conflict -- --exact --nocapture`
Expected: PASS

- [ ] **Step 3: Repeat state sync verification**

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
