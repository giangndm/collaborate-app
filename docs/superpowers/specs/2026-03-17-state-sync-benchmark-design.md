# State Sync Benchmark Design

## Goal

Add a benchmark for `collaborate-room` that measures end-to-end sync throughput for many repeated single-state changes using the public `State` API.

## Scope

This benchmark measures the full replication loop for a simple state shape:

1. mutate sender state once
2. call `poll()` until all outbound changes are drained
3. feed each change into `apply()` on the receiver
4. verify sender and receiver converge

The benchmark is intentionally focused on one mutation pattern: repeated updates to a single field many times. It does not attempt to benchmark unrelated state shapes, network transport, serialization outside the existing sync path, or multi-peer fanout.

## Recommended Approach

Use a Criterion benchmark in `crates/collaborate-room/benches/state_sync.rs`.

Why this approach:

- it produces stable, statistically useful benchmark output
- it fits standard Rust tooling with `cargo bench -p collaborate-room`
- it keeps the first benchmark small and easy to extend later

## Benchmark Shape

The benchmark should create two replicas using only the public API:

- `State::with_node_id("sender", ...)`
- `State::with_node_id("receiver", ...)`

Inside each benchmark iteration, it should:

1. initialize fresh sender and receiver states
2. run a loop of `N` repeated mutations on a single field such as `v += 1`
3. after each mutation, perform the real sync loop:
   - `while let Some(change) = sender.poll() { receiver.apply(change) }`
4. assert that sender and receiver hold equal state at the end of the iteration

This keeps the benchmark aligned with actual library behavior instead of timing isolated internals.
Fresh state creation is intentionally part of the measured end-to-end scenario; setup should stay inside the measured Criterion iteration rather than being hoisted outside as a hot-path microbenchmark optimization.
All reads and writes must stay on the public `State` surface: `with_node_id`, `poll`, `apply`, `Deref`, and `DerefMut`.

## Benchmark Cases

Start with one benchmark group named `single_field_sync` and three input sizes:

- `100`
- `1_000`
- `10_000`

These sizes are enough to show scaling without over-designing the first benchmark.

## Data Model

Define a small benchmark-only state type inside the benchmark file, for example:

- one integer field that changes every iteration
- optionally one stable string field so the state is not completely trivial

The type should derive at least:

- `Automorph`
- `Default`
- `PartialEq`
- `Eq`

Keep it minimal to isolate sync overhead from unrelated application logic.

## Correctness Rules

The benchmark should verify correctness as part of the measured scenario:

- sender and receiver must match at the end of each iteration, for example with `assert_eq!(&*sender, &*receiver)`
- benchmark code must not access `State` internals such as `.doc`
- benchmark code must use only public constructors and sync methods

If correctness assertions fail, the benchmark should fail fast rather than report misleading timing numbers.

## File Changes

- Modify `crates/collaborate-room/Cargo.toml`
  - add Criterion as a dev-dependency
  - add explicit bench target configuration with `[[bench]]`, `name = "state_sync"`, and `harness = false`
- Create `crates/collaborate-room/benches/state_sync.rs`
  - define benchmark state type
  - define sync helper used by the benchmark
  - define Criterion benchmark group and cases

## Test and Run Commands

Primary command:

```bash
cargo bench -p collaborate-room
```

Optional focused command if bench target is named explicitly:

```bash
cargo bench -p collaborate-room --bench state_sync
```

## Non-Goals

- adding production metrics code
- benchmarking internal Automerge APIs directly
- simulating transport or persistence layers
- adding multiple benchmark files before the first one proves useful

## Future Extensions

If this first benchmark is useful, later additions can include:

- sender-only `poll()` cost
- receiver-only `apply()` cost
- batch sync scenarios
- larger state payloads
- multi-peer replication benchmarks
