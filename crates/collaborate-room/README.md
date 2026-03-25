# collaborate-room

This crate currently includes two Criterion benchmarks for the `State<S>` sync
path:

- `single_field_sync`: mutate one field, then sync to a peer
- `idle_poll`: call `poll()` repeatedly when there are no local changes

The numbers below reflect the transition from the legacy `automorph` library to the new `syncable-state` library.
The benchmarks were run locally using:

```bash
cargo bench -p collaborate-room --bench state_sync
```

## Sync Benchmark

`single_field_sync` measures end-to-end sync using the public API: mutate sender
state, drain `poll()`, and apply changes on the receiver.

| Workload       | Old Time (`automorph`) | New Time (`syncable-state`) | Approx. Speedup |   New Throughput |
| -------------- | ---------------------: | --------------------------: | --------------: | ---------------: |
| `100` changes  |              `4.44 ms` |                 `166.75 µs` |          `~26x` | `599.70 Kelem/s` |
| `300` changes  |             `16.25 ms` |                 `500.12 µs` |          `~32x` | `599.86 Kelem/s` |
| `1000` changes |             `85.66 ms` |                   `1.71 ms` |          `~50x` | `584.36 Kelem/s` |

## Idle Poll Benchmark

`idle_poll` measures the steady-state no-op path: after initial setup, `poll()`
is called repeatedly with no local mutations and is expected to return `None`
every time.

| Workload       | Old Time (`automorph`) | New Time (`syncable-state`) | Approx. Speedup |   New Throughput |
| -------------- | ---------------------: | --------------------------: | --------------: | ---------------: |
| `1000` polls   |              `4.82 ms` |                   `1.75 µs` |       `~2,700x` | `572.70 Melem/s` |
| `10000` polls  |             `47.71 ms` |                  `11.74 µs` |       `~4,000x` | `851.91 Melem/s` |
| `100000` polls |            `487.47 ms` |                  `96.61 µs` |       `~5,000x` |   `1.03 Gelem/s` |

## Capacity Estimate

For a websocket-style push model, a rough server-side estimate is:

- each real user change triggers `1` sync operation
- each change also causes `100` idle `poll()` checks on the server side

Using the newly chosen baselines:

- sync baseline: `single_field_sync/1000 = 584.36 Kelem/s`
- idle baseline: `idle_poll/100000 = 1.03 Gelem/s`

Estimated cost per change:

```text
cost_per_change ~= (1 / sync_throughput) + (100 / idle_poll_throughput)
                ~= (1 / 584360) + (100 / 1030000000)
                ~= 0.00000171 + 0.00000009
                ~= 0.00000180 seconds/change
```

Estimated clients per core:

```text
clients_per_core ~= 1 / (change_rate * cost_per_change)
```

| Profile |             Change rate | Derived server work                  | Old Approx. clients | New Approx. clients/core |
| ------- | ----------------------: | ------------------------------------ | ------------------: | -----------------------: |
| Light   | `0.01 changes/user/sec` | `1 sync + 100 idle polls per change` |          `~174,000` |            `~55,000,000` |
| Normal  |  `0.1 changes/user/sec` | `1 sync + 100 idle polls per change` |           `~17,000` |             `~5,500,000` |
| Heavy   |  `1.0 changes/user/sec` | `1 sync + 100 idle polls per change` |            `~1,700` |               `~550,000` |

The `Normal` row matches the requested planning point, demonstrating a massive capacity improvement due to the transition to `syncable-state`.

## Caveats

This estimate is rough and optimistic.

- It is based on a tiny synthetic benchmark state.
- It is a single-core, single-process estimate.
- It excludes websocket framing and transport overhead.
- It excludes serialization overhead outside the measured benchmark path.
- It excludes app logic, auth, and database costs.
- It excludes room fan-out, multi-room coordination, and cross-node costs.
- Real production capacity will be lower.
