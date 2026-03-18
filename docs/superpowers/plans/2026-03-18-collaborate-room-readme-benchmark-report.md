# Collaborate Room README Benchmark Report Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `crates/collaborate-room/README.md` with a concise benchmark report covering sync performance, idle poll performance, and a rough single-core client-capacity estimate.

**Architecture:** Create a documentation-only README that copies the latest measured benchmark numbers from `cargo bench -p collaborate-room --bench state_sync`, summarizes both benchmark groups in small tables, and computes an approximate clients-per-core estimate using the agreed model: each change causes one sync plus 100 idle polls. Keep the estimate transparent, reproducible, and clearly labeled as rough.

**Tech Stack:** Markdown, Criterion benchmark output, Rust benchmark context

---

## Chunk 1: Capture benchmark inputs and write the README report

### Task 1: Re-run the benchmark and record the exact numbers

**Files:**
- Test: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Run the benchmark to get the latest local numbers**

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: PASS, with benchmark output for both `single_field_sync/...` and `idle_poll/...`

- [ ] **Step 2: Extract the exact values that will be copied into the README**

Record the latest local values for:

- `single_field_sync/100`
- `single_field_sync/300`
- `single_field_sync/1000`
- `idle_poll/1000`
- `idle_poll/10000`
- `idle_poll/100000`

### Task 2: Create `crates/collaborate-room/README.md`

**Files:**
- Create: `crates/collaborate-room/README.md`

- [ ] **Step 1: Write the README report content**

Create a concise engineering note with these sections:

1. short intro paragraph
2. `single_field_sync` benchmark table
3. `idle_poll` benchmark table
4. capacity estimate method
5. three-profile capacity table
6. caveats

Use the exact benchmark numbers from the latest local run.

For both benchmark tables, include these columns:

- workload
- approximate time
- approximate throughput

Use the exact throughput wording per table:

- `single_field_sync`: approximate throughput in elements per second
- `idle_poll`: approximate throughput in polls per second

- [ ] **Step 2: Include the estimation formula explicitly**

Document the model in plain text, for example:

```text
cost_per_change ~= (1 / sync_throughput) + (100 / idle_poll_throughput)
clients_per_core ~= 1 / (change_rate * cost_per_change)
```

State the modeling assumption explicitly: transport is websocket-style push rather than client-driven polling, so each real change is modeled as one sync plus 100 idle `poll()` checks on the server side.

Use the chosen baseline values:

- sync baseline: `single_field_sync/1000`
- idle baseline: `idle_poll/100000`

- [ ] **Step 3: Add the three user-profile rows**

Include:

- Light: `0.01 changes/user/sec`
- Normal: `0.1 changes/user/sec`
- Heavy: `1.0 changes/user/sec`

Make it explicit that `Normal` matches the requested planning point.

Require these columns in the capacity table:

- profile
- change rate
- derived server work (`1 sync + 100 idle polls per change`)
- approximate clients per core

- [ ] **Step 4: Add caveats**

Call out that the estimate is rough and optimistic because the benchmark excludes:

- single-core and single-process scaling effects
- websocket framing and transport overhead
- serialization overhead outside the measured path
- larger real-world state
- app logic, auth, and database costs
- fan-out, multi-room, and cross-node coordination

End with an explicit note that real production capacity will be lower than the README estimate.

## Chunk 2: Verify documentation accuracy

### Task 3: Re-check benchmark numbers against README content

**Files:**
- Create: `crates/collaborate-room/README.md`
- Test: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Re-run the benchmark if needed**

Run: `cargo bench -p collaborate-room --bench state_sync`
Expected: PASS

- [ ] **Step 2: Verify README numbers match the latest local output**

Read `crates/collaborate-room/README.md` and confirm every benchmark number and derived estimate matches the recorded values/formula.

- [ ] **Step 3: Verify the README stays concise and caveated**

Check that the file reads like an engineering report, not a guaranteed production sizing guide.
