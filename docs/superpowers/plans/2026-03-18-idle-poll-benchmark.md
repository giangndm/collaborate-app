# Idle Poll Benchmark Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a benchmark that measures the cost of calling `State::poll()` repeatedly when there are no local changes, representing the common steady-state idle path.

**Architecture:** Extend `crates/collaborate-room/benches/state_sync.rs` with a second Criterion benchmark group dedicated to idle polling. Each benchmark iteration should create a `State`, bring it into a steady no-pending/no-dirty condition, then repeatedly call the public `poll()` API and assert it always returns `None`.

**Tech Stack:** Rust, Criterion, `cargo bench`

---

## Chunk 1: Add idle poll benchmark

### Task 1: Benchmark steady-state `poll()` with no changes

**Files:**
- Modify: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Write the failing benchmark shape**

Add a second benchmark function and wire it into `criterion_group!`, but do not implement the body fully yet.

```rust
fn bench_idle_poll(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_poll");
    // benchmark cases go here
    group.finish();
}

criterion_group!(benches, bench_single_field_sync, bench_idle_poll);
```

- [ ] **Step 2: Run the benchmark target to verify the new shape is compiled**

Run: `cargo bench -p collaborate-room --bench state_sync --no-run`
Expected: PASS once the benchmark function is wired correctly.

- [ ] **Step 3: Write the minimal implementation**

Implement `bench_idle_poll` with separate cases such as `1_000`, `10_000`, and `100_000` poll calls. Inside each Criterion iteration:

1. create a fresh `State::with_node_id("idle", ...)`
2. do one initial `poll()` drain so the state is in a steady no-pending state
3. repeatedly call `poll()` without mutating
4. assert every call returns `None`

```rust
fn bench_idle_poll(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_poll");
    group.sample_size(10);
    group.sampling_mode(SamplingMode::Flat);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(1));

    for polls in [1_000_u64, 10_000, 100_000] {
        group.throughput(Throughput::Elements(polls));
        group.bench_with_input(BenchmarkId::from_parameter(polls), &polls, |b, &polls| {
            b.iter(|| {
                let mut state = State::with_node_id(
                    "idle",
                    BenchState {
                        label: "idle".to_string(),
                        ..BenchState::default()
                    },
                );

                while state.poll().is_some() {}

                for _ in 0..black_box(polls) {
                    assert!(state.poll().is_none());
                }
            });
        });
    }

    group.finish();
}
```

- [ ] **Step 4: Run the full benchmark**

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: Criterion reports both `single_field_sync/...` and `idle_poll/...` benchmark groups.

- [ ] **Step 5: Record the interpretation**

Summarize the idle-poll throughput separately from the active sync benchmark so future optimizations can distinguish “polling overhead” from “real sync work.”
