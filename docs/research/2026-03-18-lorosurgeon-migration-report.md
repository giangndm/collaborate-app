# Lorosurgeon Migration Report

## Question

Should `collaborate-room` switch from the current Automerge-based `State<S>` design to `lorosurgeon` for better sync performance?

## Executive Summary

Recommendation: prototype only. Do not switch this repo to `lorosurgeon` yet.

Loro itself looks promising as a future backend, but `lorosurgeon` is too new and does not fit the current `State<S>` abstraction as a drop-in replacement. The current performance issue is primarily caused by the existing Automerge sync algorithm re-exporting full history on every publish, so that bottleneck should be fixed before considering a CRDT migration.

## What `lorosurgeon` Is

- `lorosurgeon` is not the CRDT engine itself.
- Loro is the CRDT/runtime/sync engine.
- `lorosurgeon` is an Autosurgeon-style mapping layer on top of Loro that derives traits such as `Hydrate` and `Reconcile` so Rust data can round-trip to and from a `LoroDoc`.
- Its value is schema reconciliation and reduced boilerplate, not the underlying sync transport or replication protocol.

## Fit With The Current Design

Current design in `crates/collaborate-room/src/lib.rs`:

- stores plain Rust state and an Automerge doc side by side
- mutates plain Rust state directly
- calls `poll()` to diff/save/export changes
- calls `apply()` to import remote change and refresh plain state

`lorosurgeon` does not replace this directly.

Main mismatches:

- current API emits one `automerge::Change` at a time
- Loro sync is naturally based on encoded updates, imports, and version vectors
- a real migration would require redesigning `poll()` and `apply()` around Loro sync primitives rather than doing a trait-for-trait swap

Conclusion: `lorosurgeon` could support a redesigned state-mapping layer, but not a drop-in replacement of `State<S>` as it exists today.

## Performance Analysis

The current dominant bottleneck is not “Automerge is slow” in the abstract.

It is this behavior in `crates/collaborate-room/src/lib.rs:115`:

```rust
self.changes.extend(self.doc.get_changes(&[]));
```

That exports all document history on every publish.

Effects:

- after mutation 1, sender exports 1 change
- after mutation 2, sender exports 2 changes
- after mutation N, sender exports N changes
- total sync work grows roughly like `1 + 2 + ... + N`, which is quadratic

That matches the benchmark trend already observed:

- `single_field_sync/100`: about `28.7 ms`
- `single_field_sync/300`: about `237 ms`
- `single_field_sync/1000`: about `2.58 s`

So the present slowdown is mostly an algorithm problem in the current sync layer.

What Loro could change:

- raw Loro supports incremental export/import based on version vectors
- that model is a much better fit for “send only unsent updates”
- if used correctly, it could remove the current resend-all-history bottleneck

What `lorosurgeon` changes specifically:

- it may reduce unnecessary reconcile work and doc churn
- but the main performance win would come from Loro’s sync/export model, not from `lorosurgeon` itself

## Risks And Unknowns

### `lorosurgeon` maturity risk

- very new crate
- tiny adoption
- little public ecosystem evidence
- likely API churn risk

### Loro integration risk

- requires unique peer identity and version-vector tracking
- sync flow must handle imports/exports explicitly
- may need a different abstraction than the current `poll()`/`apply()` shape

### Data-model fit risk

This repo uses custom key newtypes in maps, for example:

- `HashMap<DocumentId, DocumentState>`
- `HashMap<MemberId, MemberInfo>`

Those may not map cleanly to `lorosurgeon` defaults without custom handling or data-model adjustments.

### Product risk

The sync API in this crate is still young, so switching CRDT engines now could lock in a new abstraction too early.

## Recommendation

Do not switch to `lorosurgeon` now.

Recommended sequence:

1. Fix the current Automerge sync algorithm so it exports only unsent changes.
2. Re-run the benchmark and measure the improvement.
3. If more performance is still needed, prototype raw `loro` first.
4. Only evaluate `lorosurgeon` after proving Loro fits the repo and after the crate matures more.

## Practical Next Steps

### Short term

- keep Automerge
- fix `State::poll()` so it tracks an outbound frontier instead of calling `get_changes(&[])`
- extend the benchmark to measure bytes sent and apply cost separately

### Medium term

- build a small throwaway prototype using raw `loro`
- model one narrow state type first
- test incremental update export/import and per-peer version tracking

### Later

- revisit `lorosurgeon` only if raw Loro proves valuable
- validate support for this repo’s map key shapes and trait ergonomics

## Bottom Line

Switching to `lorosurgeon` right now is unlikely to be the fastest path to better performance.

The immediate problem is the current sync algorithm, and that should be fixed first. Loro may be worth prototyping later, but `lorosurgeon` is better treated as a follow-up convenience layer, not the first migration target.
