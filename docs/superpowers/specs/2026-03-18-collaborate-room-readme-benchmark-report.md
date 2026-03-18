# Collaborate Room README Benchmark Report Design

## Goal

Create `crates/collaborate-room/README.md` with a benchmark report that documents the current sync and idle-poll benchmark results and gives a rough single-core client-capacity estimate.

## Scope

The README update should cover:

- the current `single_field_sync` benchmark results
- the current `idle_poll` benchmark results
- a simple estimation model for websocket-driven usage where each real change triggers one sync plus 100 idle `poll()` checks
- a client-capacity table for three user profiles

This is a documentation-only change. It does not change runtime behavior or the benchmark code.

## Benchmark Data To Include

Use the exact values from the latest local `cargo bench -p collaborate-room --bench state_sync` run and copy those measured numbers into the README tables.

Document both benchmark groups:

### `single_field_sync`

- `100` changes
- `300` changes
- `1000` changes

For each row, include:

- approximate time
- approximate throughput in elements per second

### `idle_poll`

- `1000` polls
- `10000` polls
- `100000` polls

For each row, include:

- approximate time
- approximate throughput in polls per second

## Capacity Estimate Model

The README should explain the estimate with a short formula.

Assumption model:

- transport is websocket-style push, not client-driven polling
- each user change triggers:
  - `1` sync operation
  - `100` idle `poll()` checks on the server side

Use the benchmark numbers to derive an approximate per-change budget.

Suggested presentation:

- choose the `single_field_sync/1000` number as the sync throughput baseline
- choose the `idle_poll/100000` number as the idle-poll throughput baseline
- estimate per-change cost as:

```text
cost_per_change ~= (1 / sync_throughput) + (100 / idle_poll_throughput)
```

- estimate single-core client capacity as:

```text
clients_per_core ~= 1 / (change_rate * cost_per_change)
```

The README should present the final table rather than forcing readers to do the math themselves.

## User Profiles Table

Add a table with three rows:

- Light: `0.01 changes/user/sec`
- Normal: `0.1 changes/user/sec`
- Heavy: `1.0 changes/user/sec`

For each row, include:

- change rate
- assumed derived server work (`1 sync + 100 idle polls per change`)
- approximate clients per core

Explicitly note that the `Normal` row (`0.1 changes/user/sec`) matches the user’s requested planning point.

## Accuracy And Caveats

The README should clearly label the estimate as rough and optimistic.

Include caveats such as:

- benchmark uses a tiny synthetic state
- benchmark is single-core and single-process
- no websocket framing, serialization overhead outside the measured path, database, auth, or business logic is included
- no multi-room fan-out or cross-node coordination is included
- real production capacity will be lower

## Writing Style

Keep the report concise and practical:

- one short intro paragraph
- two small benchmark tables
- one capacity table
- a short caveats section

The README should read like an engineering note, not a marketing claim.
