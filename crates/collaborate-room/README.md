# collaborate-room

This crate currently includes two Criterion benchmarks for the `State<S>` sync
path:

- `single_field_sync`: mutate one field, then sync to a peer
- `idle_poll`: call `poll()` repeatedly when there are no local changes

The numbers below come from the latest local run of:

```bash
cargo bench -p collaborate-room --bench state_sync
```

## Sync Benchmark

`single_field_sync` measures end-to-end sync using the public API: mutate sender
state, drain `poll()`, and apply changes on the receiver.

| Workload       | Approx. time | Approx. throughput in elements/sec |
| -------------- | -----------: | ---------------------------------: |
| `100` changes  |    `4.44 ms` |                    `22.50 Kelem/s` |
| `300` changes  |   `16.25 ms` |                    `18.46 Kelem/s` |
| `1000` changes |   `85.66 ms` |                    `11.67 Kelem/s` |

## Idle Poll Benchmark

`idle_poll` measures the steady-state no-op path: after initial setup, `poll()`
is called repeatedly with no local mutations and is expected to return `None`
every time.

| Workload       | Approx. time | Approx. throughput in polls/sec |
| -------------- | -----------: | ------------------------------: |
| `1000` polls   |    `4.82 ms` |                `207.41 Kelem/s` |
| `10000` polls  |   `47.71 ms` |                `209.62 Kelem/s` |
| `100000` polls |  `487.47 ms` |                `205.14 Kelem/s` |

## Capacity Estimate

For a websocket-style push model, a rough server-side estimate is:

- each real user change triggers `1` sync operation
- each change also causes `100` idle `poll()` checks on the server side

Using the chosen baselines:

- sync baseline: `single_field_sync/1000 = 11.67 Kelem/s`
- idle baseline: `idle_poll/100000 = 205.14 Kelem/s`

Estimated cost per change:

```text
cost_per_change ~= (1 / sync_throughput) + (100 / idle_poll_throughput)
                ~= (1 / 11673) + (100 / 205140)
                ~= 0.000573 seconds/change
```

Estimated clients per core:

```text
clients_per_core ~= 1 / (change_rate * cost_per_change)
```

| Profile |             Change rate | Derived server work                  | Approx. clients/core |
| ------- | ----------------------: | ------------------------------------ | -------------------: |
| Light   | `0.01 changes/user/sec` | `1 sync + 100 idle polls per change` |           `~174,000` |
| Normal  |  `0.1 changes/user/sec` | `1 sync + 100 idle polls per change` |            `~17,000` |
| Heavy   |  `1.0 changes/user/sec` | `1 sync + 100 idle polls per change` |             `~1,700` |

The `Normal` row matches the requested planning point.

## Caveats

This estimate is rough and optimistic.

- It is based on a tiny synthetic benchmark state.
- It is a single-core, single-process estimate.
- It excludes websocket framing and transport overhead.
- It excludes serialization overhead outside the measured benchmark path.
- It excludes app logic, auth, and database costs.
- It excludes room fan-out, multi-room coordination, and cross-node costs.
- Real production capacity will be lower.
