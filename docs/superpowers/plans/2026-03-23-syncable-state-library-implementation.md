# Syncable State Library Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first production-quality `syncable-state` Rust library slice for this repo, including typed runtime primitives, batch-based change capture, core sync containers, a minimal derive crate, and full wiring into `collaborate-room` so the crate no longer depends on `automerge` or `automorph`.

**Architecture:** Introduce two new workspace crates: `syncable-state` for runtime types and `syncable-state-derive` for proc-macro expansion. Implement the authoritative-writer v1 first: `ChangeCtx`/`BatchTx`, snapshot and delta models, typed containers (`String`, `Text`, `Counter`, `Vec`, `Map`), then layer a minimal `#[derive(SyncableState)]` that generates schema, snapshot conversion, and path routing. After the library is proven in isolation, rewire `collaborate-room` to use it end to end and remove the old `automerge`/`automorph`-based sync path.

**Tech Stack:** Rust 2024, workspace crates, proc macros (`syn`, `quote`, `proc-macro2`), `serde`, `thiserror`, `trybuild`, `cargo test`

---

## File Map

### New workspace crates

- Create: `crates/syncable-state/Cargo.toml`
- Create: `crates/syncable-state/src/lib.rs`
- Create: `crates/syncable-state/src/error.rs`
- Create: `crates/syncable-state/src/path.rs`
- Create: `crates/syncable-state/src/change.rs`
- Create: `crates/syncable-state/src/context.rs`
- Create: `crates/syncable-state/src/schema.rs`
- Create: `crates/syncable-state/src/snapshot.rs`
- Create: `crates/syncable-state/src/traits.rs`
- Create: `crates/syncable-state/src/containers/mod.rs`
- Create: `crates/syncable-state/src/containers/string.rs`
- Create: `crates/syncable-state/src/containers/counter.rs`
- Create: `crates/syncable-state/src/containers/text.rs`
- Create: `crates/syncable-state/src/containers/vec.rs`
- Create: `crates/syncable-state/src/containers/map.rs`
- Create: `crates/syncable-state/tests/context_batches.rs`
- Create: `crates/syncable-state/tests/string_container.rs`
- Create: `crates/syncable-state/tests/counter_container.rs`
- Create: `crates/syncable-state/tests/text_container.rs`
- Create: `crates/syncable-state/tests/vec_container.rs`
- Create: `crates/syncable-state/tests/map_container.rs`
- Create: `crates/syncable-state/tests/document_flow.rs`
- Create: `crates/syncable-state/tests/runtime_apply.rs`

- Create: `crates/syncable-state-derive/Cargo.toml`
- Create: `crates/syncable-state-derive/src/lib.rs`
- Create: `crates/syncable-state-derive/src/attrs.rs`
- Create: `crates/syncable-state-derive/src/parse.rs`
- Create: `crates/syncable-state-derive/src/validate.rs`
- Create: `crates/syncable-state-derive/src/snapshot.rs`
- Create: `crates/syncable-state-derive/src/schema.rs`
- Create: `crates/syncable-state-derive/src/expand.rs`
- Create: `crates/syncable-state-derive/src/diagnostics.rs`
- Create: `crates/syncable-state-derive/tests/trybuild.rs`
- Create: `crates/syncable-state-derive/tests/ui/derive_ok.rs`
- Create: `crates/syncable-state-derive/tests/ui/duplicate_id.rs`
- Create: `crates/syncable-state-derive/tests/ui/duplicate_id.stderr`
- Create: `crates/syncable-state-derive/tests/ui/tuple_struct.rs`
- Create: `crates/syncable-state-derive/tests/ui/tuple_struct.stderr`
- Create: `crates/syncable-state-derive/tests/ui/rename_conflict.rs`
- Create: `crates/syncable-state-derive/tests/ui/rename_conflict.stderr`

### Workspace and integration points

- Modify: `Cargo.toml`
- Modify: `docs/syncable-state-lib.md`
- Modify: `crates/collaborate-room/Cargo.toml`
- Modify: `crates/collaborate-room/src/lib.rs`
- Modify: `crates/collaborate-room/src/sync.rs`
- Modify: `crates/collaborate-room/src/apps.rs`
- Modify: `crates/collaborate-room/src/apps/document.rs`
- Modify: `crates/collaborate-room/src/room.rs`
- Modify: `crates/collaborate-room/src/types.rs`
- Modify: `crates/collaborate-room/benches/state_sync.rs`
- Create or Modify: `crates/collaborate-room/tests/syncable_state_document.rs`

---

## Chunk 1: Scaffold the runtime crate and lock the batch model

### Task 1: Add workspace crates and dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/syncable-state/Cargo.toml`
- Create: `crates/syncable-state-derive/Cargo.toml`

- [ ] **Step 1: Write a failing workspace smoke test**

Create `crates/syncable-state/tests/context_batches.rs` with a minimal import of the future public API:

```rust
use syncable_state::{BatchTx, ChangeCtx};

#[test]
fn change_ctx_public_api_exists() {
    let _ctx = ChangeCtx::new("test-replica");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test context_batches`
Expected: FAIL because the crate and symbols do not exist yet.

- [ ] **Step 3: Add the new workspace crates minimally**

Create both crates with minimal manifests. Wire `syncable-state-derive` as an optional or normal dependency of `syncable-state` so runtime re-exports the derive macro later.

- [ ] **Step 4: Run the test to verify the crate now resolves but behavior is still missing**

Run: `cargo test -p syncable-state --test context_batches`
Expected: FAIL with missing constructor or API details, not missing crate errors.

### Task 2: Implement the core runtime model (`path`, `change`, `snapshot`, `traits`)

**Files:**
- Create: `crates/syncable-state/src/lib.rs`
- Create: `crates/syncable-state/src/context.rs`
- Create: `crates/syncable-state/src/change.rs`
- Create: `crates/syncable-state/src/path.rs`
- Create: `crates/syncable-state/src/snapshot.rs`
- Create: `crates/syncable-state/src/schema.rs`
- Create: `crates/syncable-state/src/traits.rs`
- Create: `crates/syncable-state/src/error.rs`
- Test: `crates/syncable-state/tests/context_batches.rs`

- [ ] **Step 1: Expand the failing test to lock batch semantics**

Add tests for:

```rust
#[test]
fn committed_batch_advances_seq_by_one() {
    let mut ctx = ChangeCtx::new("r1");
    let mut batch = ctx.begin_batch().unwrap();
    batch.push(ChangeEnvelope::new(
        SyncPath::from_field("title"),
        ChangeOp::String(StringOp::Set("A".into())),
    ));
    let committed = batch.commit().unwrap();

    assert_eq!(committed.from_seq, 0);
    assert_eq!(committed.to_seq, 1);
    assert_eq!(committed.changes.len(), 1);
}

#[test]
fn poll_returns_committed_batches_in_order() {
    let mut ctx = ChangeCtx::new("r1");
    let mut batch1 = ctx.begin_batch().unwrap();
    batch1.push(ChangeEnvelope::new(
        SyncPath::from_field("title"),
        ChangeOp::String(StringOp::Set("A".into())),
    ));
    batch1.commit().unwrap();

    let mut batch2 = ctx.begin_batch().unwrap();
    batch2.push(ChangeEnvelope::new(
        SyncPath::from_field("revision"),
        ChangeOp::Counter(CounterOp::Increment(1)),
    ));
    batch2.commit().unwrap();

    assert_eq!(ctx.poll().unwrap().to_seq, 1);
    assert_eq!(ctx.poll().unwrap().to_seq, 2);
    assert!(ctx.poll().is_none());
}
```

- [ ] **Step 2: Run the test to verify it fails for the expected missing behavior**

Run: `cargo test -p syncable-state --test context_batches`
Expected: FAIL because the batch, path, envelope, and op types are incomplete.

- [ ] **Step 3: Implement the minimal runtime model**

Add:

- `ReplicaId`
- `PathSegment` + `SyncPath`
- `SnapshotValue`
- `ChangeEnvelope`
- `ChangeOp` with `StringOp`, `CounterOp`, and placeholder child enums
- `SnapshotBundle<T>`
- `DeltaBatch`
- explicit batch identity keyed by `(replica_id, to_seq)` for authoritative-writer idempotency
- `FieldSchema`, `FieldKind`, and a minimal `StateSchema`
- `SyncableState`, `ApplyPath`, and a minimal container-facing trait surface
- `ChangeCtx::new`, `begin_batch`, `poll`
- `BatchTx::push`, `commit`

- [ ] **Step 4: Run the focused test again**

Run: `cargo test -p syncable-state --test context_batches`
Expected: PASS

- [ ] **Step 5: Run crate tests to verify no regressions in the new crate**

Run: `cargo test -p syncable-state`
Expected: PASS

### Task 3: Implement runtime-level remote apply and continuity enforcement

**Files:**
- Modify: `crates/syncable-state/src/lib.rs`
- Modify: `crates/syncable-state/src/context.rs`
- Modify: `crates/syncable-state/src/traits.rs`
- Modify: `crates/syncable-state/src/error.rs`
- Create: `crates/syncable-state/tests/runtime_apply.rs`

- [ ] **Step 1: Write the failing runtime apply tests**

Cover:

- applying a batch with `from_seq == local_seq` advances local state
- applying a batch with `from_seq > local_seq` returns `SyncError::GapDetected`
- applying a batch with `from_seq < local_seq` is ignored idempotently only when it is a true replay of the same authoritative batch; stale conflicting or wrong-authority input fails clearly
- remote apply does not re-queue the same batch into local pending outbound deltas

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test runtime_apply`
Expected: FAIL because runtime-level import semantics do not exist yet.

- [ ] **Step 3: Implement the minimal authoritative-writer runtime contract**

Add a small runtime trait or façade with explicit methods such as:

```rust
fn current_seq(&self) -> u64;
fn snapshot(&self) -> SnapshotBundle<Self::Snapshot>;
fn apply_remote(&mut self, batch: DeltaBatch) -> Result<(), SyncError>;
```

Be explicit that v1 uses one authoritative sequence owner and that duplicate detection is keyed by `(replica_id, to_seq)` rather than only `to_seq`.

- [ ] **Step 4: Run the runtime apply tests**

Run: `cargo test -p syncable-state --test runtime_apply`
Expected: PASS

- [ ] **Step 5: Re-run the crate tests**

Run: `cargo test -p syncable-state`
Expected: PASS

## Chunk 2: Implement scalar containers first (`SyncableString`, `SyncableCounter`)

### Task 4: Add `SyncableString`

**Files:**
- Create: `crates/syncable-state/src/traits.rs`
- Create: `crates/syncable-state/src/containers/mod.rs`
- Create: `crates/syncable-state/src/containers/string.rs`
- Test: `crates/syncable-state/tests/string_container.rs`

- [ ] **Step 1: Write the failing string container tests**

Cover:

- `set()` updates local value and emits `ChangeOp::String(StringOp::Set(...))`
- `clear()` emits `StringOp::Clear`
- snapshot returns plain `String`
- remote apply on the root field path updates materialized state

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test string_container`
Expected: FAIL because `SyncableString` and container traits do not exist yet.

- [ ] **Step 3: Implement the minimal string container**

Add a small trait split, e.g. `SyncContainer` / `ApplyPath`, and implement:

- `SyncableString::new`
- `SyncableString::snapshot`
- `SyncableString::set(&mut BatchTx, value)`
- `SyncableString::clear(&mut BatchTx)`
- `apply_path_tail(&mut self, path_tail, op)`

- [ ] **Step 4: Run the string tests**

Run: `cargo test -p syncable-state --test string_container`
Expected: PASS

### Task 5: Add `SyncableCounter`

**Files:**
- Create: `crates/syncable-state/src/containers/counter.rs`
- Test: `crates/syncable-state/tests/counter_container.rs`

- [ ] **Step 1: Write the failing counter tests**

Cover:

- `increment()` and `decrement()` update materialized value
- emitted changes use `ChangeOp::Counter`
- snapshot returns `i64`
- remote apply handles stale duplicate batch idempotently at the runtime layer but container apply itself stays deterministic

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test counter_container`
Expected: FAIL because the counter container does not exist yet.

- [ ] **Step 3: Implement the minimal counter container**

Add:

- `SyncableCounter::new`
- `increment(&mut BatchTx, by)`
- `decrement(&mut BatchTx, by)`
- `snapshot()`
- `apply_path_tail()`

- [ ] **Step 4: Run the counter tests**

Run: `cargo test -p syncable-state --test counter_container`
Expected: PASS

- [ ] **Step 5: Re-run the runtime crate tests**

Run: `cargo test -p syncable-state`
Expected: PASS

## Chunk 3: Add structured containers (`Text`, `Vec`, `Map`) and document flow proof

### Task 6: Add `SyncableText`

**Files:**
- Create: `crates/syncable-state/src/containers/text.rs`
- Test: `crates/syncable-state/tests/text_container.rs`

- [ ] **Step 1: Write the failing text tests**

Cover:

- `splice()` updates materialized text
- emitted op is `TextOp::Splice`
- `clear()` emits `TextOp::Clear`
- snapshot returns plain `String`

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test text_container`
Expected: FAIL because `SyncableText` does not exist yet.

- [ ] **Step 3: Implement the minimal text container for v1**

Use a simple string-backed implementation that honors the public batch/path contract first. Do not over-design the internal text CRDT yet.

- [ ] **Step 4: Run the text tests**

Run: `cargo test -p syncable-state --test text_container`
Expected: PASS

### Task 7: Add `SyncableVec<T>` with stable identity routing

**Files:**
- Create: `crates/syncable-state/src/containers/vec.rs`
- Test: `crates/syncable-state/tests/vec_container.rs`

- [ ] **Step 1: Write the failing vector tests**

Cover:

- insert emits `ListOp::Insert`
- delete by id emits `ListOp::Delete`
- `get_mut(&id)` resolves by stable identity, not position
- nested child updates use path shape `docs[id="..."]....`
- snapshot returns a `Vec<T::Snapshot>` in stable order
- remote insert carries a full child `SnapshotValue` payload that materializes one child item in a single step

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test vec_container`
Expected: FAIL because vector identity routing does not exist yet.

- [ ] **Step 3: Implement insert/delete with identity and full-snapshot child payloads**

Use an order list plus an id-to-item map internally. For v1, `ListOp::Insert` must carry the full child snapshot payload needed to materialize the item remotely in one apply step.

- [ ] **Step 4: Run the vector tests to verify insert/delete now work**

Run: `cargo test -p syncable-state --test vec_container`
Expected: FAIL only on nested path or remaining routing behavior.

- [ ] **Step 5: Implement nested child routing and snapshot export**

Add `get_mut(&id)`, path-tail routing, and `Vec<T::Snapshot>` export.

- [ ] **Step 6: Run the vector tests**

Run: `cargo test -p syncable-state --test vec_container`
Expected: PASS

### Task 8: Add `SyncableMap<String, V>`

**Files:**
- Create: `crates/syncable-state/src/containers/map.rs`
- Test: `crates/syncable-state/tests/map_container.rs`

- [ ] **Step 1: Write the failing map tests**

Cover:

- insert/replace/remove emit `MapOp`
- nested child routing uses `PathSegment::Key`
- snapshot returns stable serialized values
- insert and replace use full child `SnapshotValue` payloads with schema validation on apply

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test map_container`
Expected: FAIL because the map container does not exist yet.

- [ ] **Step 3: Implement insert/remove with schema-backed payload validation**

Use `BTreeMap<String, V>` for deterministic ordering in tests and snapshots. Make map apply reject payloads that cannot materialize into the target child schema.

- [ ] **Step 4: Run the map tests to verify insert/remove now work**

Run: `cargo test -p syncable-state --test map_container`
Expected: FAIL only on replace or nested routing if those remain missing.

- [ ] **Step 5: Implement replace and nested child routing**

- [ ] **Step 6: Run the map tests**

Run: `cargo test -p syncable-state --test map_container`
Expected: PASS

### Task 9: Prove the typed document mutation flow end to end without derive

**Files:**
- Test: `crates/syncable-state/tests/document_flow.rs`

- [ ] **Step 1: Write the failing integration-style test**

Define a small manual `DocumentState` test type that uses the containers directly and prove:

- delete by id emits a `ListOp::Delete`
- rename emits a string set and counter increment in one committed batch
- snapshot sequence and polled delta sequence remain aligned

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p syncable-state --test document_flow`
Expected: FAIL until the containers work together coherently.

- [ ] **Step 3: Implement the minimum glue needed in runtime traits**

Add whatever small trait helpers are still missing, but keep them generic and library-worthy.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p syncable-state --test document_flow`
Expected: PASS

## Chunk 4: Implement the derive crate minimally but rigorously

### Task 10: Add compile-fail coverage for `#[derive(SyncableState)]`

**Files:**
- Create: `crates/syncable-state-derive/tests/trybuild.rs`
- Create: `crates/syncable-state-derive/tests/ui/derive_ok.rs`
- Create: `crates/syncable-state-derive/tests/ui/duplicate_id.rs`
- Create: `crates/syncable-state-derive/tests/ui/tuple_struct.rs`
- Create: `crates/syncable-state-derive/tests/ui/rename_conflict.rs`

- [ ] **Step 1: Write the trybuild harness and failing UI cases**

Use `trybuild` to assert:

- valid derive compiles
- duplicate `#[sync(id)]` fails
- tuple struct derive fails
- renamed field collision fails

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p syncable-state-derive --test trybuild`
Expected: FAIL because the macro crate is not implemented yet.

- [ ] **Step 3: Prepare checked-in stderr baselines in the standard trybuild flow**

Run: `TRYBUILD=overwrite cargo test -p syncable-state-derive --test trybuild`
Expected: `.stderr` files are generated or refreshed for the failing UI cases.

### Task 11: Implement parse, validate, snapshot generation, and routing expansion

**Files:**
- Create: `crates/syncable-state-derive/src/lib.rs`
- Create: `crates/syncable-state-derive/src/attrs.rs`
- Create: `crates/syncable-state-derive/src/parse.rs`
- Create: `crates/syncable-state-derive/src/validate.rs`
- Create: `crates/syncable-state-derive/src/snapshot.rs`
- Create: `crates/syncable-state-derive/src/schema.rs`
- Create: `crates/syncable-state-derive/src/expand.rs`
- Create: `crates/syncable-state-derive/src/diagnostics.rs`
- Test: `crates/syncable-state-derive/tests/trybuild.rs`

- [ ] **Step 1: Add a runtime behavior test for the generated code before the macro is implemented**

In `crates/syncable-state/tests/document_flow.rs`, add one derive-backed test that will later assert generated snapshot conversion and field routing actually work at runtime.

- [ ] **Step 2: Run that test and verify it fails because the derive macro is not ready yet**

Run: `cargo test -p syncable-state --test document_flow`
Expected: FAIL because generated impls do not exist yet.

- [ ] **Step 3: Implement attribute parsing and state parsing**

Support only named-field structs in v1. Parse `#[sync(id)]`, `#[sync(skip)]`, `#[sync(rename = ...)]`, and `#[sync(with = ...)]`.

- [ ] **Step 4: Run the trybuild tests to verify they still fail, but for the next missing behavior**

Run: `cargo test -p syncable-state-derive --test trybuild`
Expected: FAIL because validation and expansion are not complete yet.

- [ ] **Step 5: Implement validation only**

Get duplicate id, tuple struct rejection, and rename conflict failures stable first.

- [ ] **Step 6: Regenerate the stderr baselines if diagnostics changed**

Run: `TRYBUILD=overwrite cargo test -p syncable-state-derive --test trybuild`
Expected: updated `.stderr` files reflect the now-stable diagnostics.

- [ ] **Step 7: Implement snapshot generation and `impl SyncableState` expansion**

Generate:

- snapshot type
- schema metadata
- `impl SyncableState`
- path routing for fields
- identity accessor if an id field exists

- [ ] **Step 8: Run the derive crate tests**

Run: `cargo test -p syncable-state-derive --test trybuild`
Expected: PASS

- [ ] **Step 9: Re-run both crate test suites**

Run: `cargo test -p syncable-state && cargo test -p syncable-state-derive`
Expected: PASS

## Chunk 5: Rewire `collaborate-room` and remove the old Automerge path

### Task 12: Replace the `collaborate-room` sync contract with the new typed batch transport

**Files:**
- Modify: `crates/collaborate-room/src/lib.rs`
- Modify: `crates/collaborate-room/src/sync.rs`
- Modify: `crates/collaborate-room/src/apps.rs`
- Modify: `crates/collaborate-room/src/room.rs`
- Create or Modify: `crates/collaborate-room/tests/syncable_state_document.rs`

- [ ] **Step 1: Write the failing `collaborate-room` transport contract tests**

Lock the new `SyncChange` surface first. The test should prove that `poll()` and `apply()` now exchange the new typed batch payload instead of `automerge::Change`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p collaborate-room --test syncable_state_document`
Expected: FAIL because the crate still exposes the Automerge-based contract.

- [ ] **Step 3: Replace `SyncChange`, `State`, and `StateC` in `crates/collaborate-room/src/sync.rs` with adapters over `syncable-state`**

Keep the public surface as small as possible. The goal is to stabilize the crate's own sync abstraction before migrating individual room/app states.

- [ ] **Step 4: Run the focused test again**

Run: `cargo test -p collaborate-room --test syncable_state_document`
Expected: FAIL only because the document and room states still use old data modeling.

### Task 13: Wire the document flow, room flow, and benchmark onto `syncable-state`

**Files:**
- Modify or Create: `crates/collaborate-room/tests/syncable_state_document.rs`
- Modify: `crates/collaborate-room/Cargo.toml`
- Modify: `crates/collaborate-room/src/lib.rs`
- Modify: `crates/collaborate-room/src/sync.rs`
- Modify: `crates/collaborate-room/src/apps.rs`
- Modify: `crates/collaborate-room/src/apps/document.rs`
- Modify: `crates/collaborate-room/src/room.rs`
- Modify: `crates/collaborate-room/src/types.rs`
- Modify: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Write the failing integration test**

Use:

```rust
#[derive(SyncableState)]
struct DocumentAppState {
    docs: SyncableVec<DocumentState>,
}

#[derive(SyncableState)]
struct DocumentState {
    #[sync(id)]
    id: String,
    title: SyncableString,
    content: SyncableText,
    revision: SyncableCounter,
}
```

Then assert:

- deleting a doc emits the correct path and `ListOp::Delete`
- renaming emits one `DeltaBatch` with both title and revision changes
- snapshot and delta sequence line up after mutations
- room membership state still syncs through the new typed runtime
- the benchmark compiles against the new sync API

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p collaborate-room --test syncable_state_document`
Expected: FAIL until the runtime and derive crate interoperate correctly.

- [ ] **Step 3: Add the minimum integration wiring**

Wire the new crate dependency and replace the document sync path in `collaborate-room` with the new typed runtime. Do not leave the old Automerge-backed sync path active in parallel unless a very small compatibility shim is unavoidable for the migration.

- [ ] **Step 4: Run the integration test**

Run: `cargo test -p collaborate-room --test syncable_state_document`
Expected: PASS

### Task 14: Remove `automerge` and `automorph` from `collaborate-room`

**Files:**
- Modify: `crates/collaborate-room/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `crates/collaborate-room/src/lib.rs`
- Modify: `crates/collaborate-room/src/sync.rs`
- Modify: `crates/collaborate-room/src/apps.rs`
- Modify: `crates/collaborate-room/src/apps/document.rs`
- Modify: `crates/collaborate-room/src/room.rs`
- Modify: `crates/collaborate-room/src/types.rs`
- Modify: `crates/collaborate-room/benches/state_sync.rs`

- [ ] **Step 1: Write or expand the failing crate-level tests so they cover the migrated sync path**

Make sure existing `collaborate-room` tests and the new typed document test are enough to prove the crate still syncs correctly after the old dependencies are removed.

- [ ] **Step 2: Run the focused crate tests before removing dependencies**

Run: `cargo test -p collaborate-room`
Expected: PASS on the migrated implementation before dependency cleanup.

- [ ] **Step 3: Remove `automerge` and `automorph` dependencies from `crates/collaborate-room/Cargo.toml` and the workspace root if they are no longer used anywhere**

If another crate still needs them, remove only the `collaborate-room` usage now and leave a note in the doc update step.

- [ ] **Step 4: Delete or rewrite any leftover sync code that still relies on the removed dependencies**

Keep the public API coherent; do not leave dead types like `SyncChange = automerge::Change` behind.

- [ ] **Step 5: Run the crate tests again to verify the cleanup is real**

Run: `cargo test -p collaborate-room`
Expected: PASS without `automerge` / `automorph` in `collaborate-room`.

### Task 15: Final verification and doc update

**Files:**
- Modify: `docs/syncable-state-lib.md`

- [ ] **Step 1: Update the design doc with any implementation-delivered clarifications**

Only update places where the actual crate names, type names, or validated trade-offs differ from the design.

- [ ] **Step 2: Run the full workspace tests from the worktree**

Run: `cargo test --workspace`
Expected: PASS

- [ ] **Step 3: Run a workspace build to confirm final compile health**

Run: `cargo build`
Expected: PASS

- [ ] **Step 4: Review the git diff for only intended files**

Run: `git status --short && git diff --stat`
Expected: only the planned new crates, tests, and doc updates appear.
