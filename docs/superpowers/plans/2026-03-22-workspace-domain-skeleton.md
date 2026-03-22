# Workspace Domain Skeleton Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a compile-safe, hexagonal `workspace domain` skeleton in `crates/core-domain` with strong typed models, action-based permissions, guards, and a small set of test contracts for the most important flows.

**Architecture:** Keep `workspace domain` isolated inside `crates/core-domain/src/domain/workspace/`. Domain types and invariants live in `types/` and `entity/`, permission derivation lives in `guards/`, dependency boundaries live in `ports/`, and orchestration lives in `service/`. The skeleton must compile, avoid stringly typed domain contracts, and leave detailed business logic as documented TODOs rather than hidden implementation guesses.

**Tech Stack:** Rust, `thiserror`, `derive_more`, `tokio`, `futures`, hexagonal architecture, unit tests as test contracts.

---

## Chunk 1: Prepare crate and module layout

### Task 1: Add domain dependencies and reorganize workspace module tree

**Files:**
- Modify: `crates/core-domain/Cargo.toml`
- Modify: `crates/core-domain/src/lib.rs`
- Modify: `crates/core-domain/src/domain.rs`
- Modify: `crates/core-domain/src/domain/workspace/mod.rs`
- Create: `crates/core-domain/src/domain/workspace/types/mod.rs`
- Create: `crates/core-domain/src/domain/workspace/entity/mod.rs`
- Create: `crates/core-domain/src/domain/workspace/guards/mod.rs`
- Create: `crates/core-domain/src/domain/workspace/ports/mod.rs`
- Create: `crates/core-domain/src/domain/workspace/service/mod.rs`
- Test: `cargo test -p core-domain`

- [ ] Step 1: Add `thiserror` and `derive_more` to `crates/core-domain/Cargo.toml`
- [ ] Step 2: Replace the flat `workspace/{types,service,repo}.rs` exposure with nested module folders described in `docs/implement_workspace_domain.md`
- [ ] Step 3: Export only the workspace domain surface needed by later tasks
- [ ] Step 4: Run `cargo test -p core-domain` to verify the empty skeleton compiles

### Task 2: Remove obsolete flat workspace files

**Files:**
- Delete: `crates/core-domain/src/domain/workspace/types.rs`
- Delete: `crates/core-domain/src/domain/workspace/service.rs`
- Delete: `crates/core-domain/src/domain/workspace/repo.rs`
- Test: `cargo test -p core-domain`

- [ ] Step 1: Delete obsolete flat files after the nested module tree is in place
- [ ] Step 2: Run `cargo test -p core-domain` again to verify module wiring is still correct

## Chunk 2: Typed models, entities, and guards

### Task 3: Add workspace typed models and errors

**Files:**
- Create: `crates/core-domain/src/domain/workspace/types/ids.rs`
- Create: `crates/core-domain/src/domain/workspace/types/user.rs`
- Create: `crates/core-domain/src/domain/workspace/types/membership.rs`
- Create: `crates/core-domain/src/domain/workspace/types/permissions.rs`
- Create: `crates/core-domain/src/domain/workspace/types/status.rs`
- Create: `crates/core-domain/src/domain/workspace/types/policy.rs`
- Create: `crates/core-domain/src/domain/workspace/types/credentials.rs`
- Create: `crates/core-domain/src/domain/workspace/types/sync.rs`
- Create: `crates/core-domain/src/domain/workspace/types/errors.rs`
- Test: `crates/core-domain/src/domain/workspace/types/tests.rs` or inline unit tests

- [ ] Step 1: Write failing test contracts for key typed models: ids, roles, action permissions, and `WorkspaceError` formatting/context
- [ ] Step 2: Run `cargo test -p core-domain workspace::types` or the narrowest available command to verify failure
- [ ] Step 3: Implement the minimal typed models and `thiserror` enums to make the tests pass
- [ ] Step 4: Run `cargo test -p core-domain` and verify the type contract tests pass

### Task 4: Add workspace entities

**Files:**
- Create: `crates/core-domain/src/domain/workspace/entity/workspace.rs`
- Create: `crates/core-domain/src/domain/workspace/entity/user.rs`
- Create: `crates/core-domain/src/domain/workspace/entity/membership.rs`
- Test: unit tests colocated with entity files

- [ ] Step 1: Write failing tests for baseline entity construction and the most important invariant-preserving transitions (`activate/suspend`, membership role storage)
- [ ] Step 2: Run the narrow test command and confirm failure
- [ ] Step 3: Add minimal entities with doc comments, meaningful TODO notes, and only the smallest real behavior needed by tests
- [ ] Step 4: Run `cargo test -p core-domain` and verify entity tests pass

### Task 5: Add guards and permission conversion contracts

**Files:**
- Create: `crates/core-domain/src/domain/workspace/guards/workspace_member_guard.rs`
- Create: `crates/core-domain/src/domain/workspace/guards/super_admin_guard.rs`
- Test: unit tests colocated with guard files

- [ ] Step 1: Write failing tests covering baseline permission derivation: member read, owner/admin invite or update, and super admin conversion into workspace-scoped permissions
- [ ] Step 2: Run the narrow test command and confirm failure
- [ ] Step 3: Implement the guard skeletons and `TryFrom`/`From` conversions needed for those tests
- [ ] Step 4: Run `cargo test -p core-domain` and verify guard tests pass

## Chunk 3: Ports, services, and service-level test contracts

### Task 6: Add repository and secret store ports

**Files:**
- Create: `crates/core-domain/src/domain/workspace/ports/workspace_repository.rs`
- Create: `crates/core-domain/src/domain/workspace/ports/user_repository.rs`
- Create: `crates/core-domain/src/domain/workspace/ports/membership_repository.rs`
- Create: `crates/core-domain/src/domain/workspace/ports/secret_store.rs`

- [ ] Step 1: Define intent-level traits only; do not add adapter logic
- [ ] Step 2: Ensure all trait methods use typed ids, typed entities, and typed errors where appropriate
- [ ] Step 3: Run `cargo test -p core-domain` to verify trait contracts compile with the rest of the module tree

### Task 7: Add workspace services and sync export service skeleton

**Files:**
- Create: `crates/core-domain/src/domain/workspace/service/workspace_service.rs`
- Create: `crates/core-domain/src/domain/workspace/service/sync_service.rs`
- Test: `crates/core-domain/src/domain/workspace/service/tests.rs` or colocated service tests

- [ ] Step 1: Write failing test contracts for the service surface only: methods must require typed permissions and typed ids, and sync export must produce `WorkspaceSyncPayload` from repository + secret store inputs
- [ ] Step 2: Run the narrow test command and confirm failure
- [ ] Step 3: Implement minimal service skeleton structs with short orchestrator bodies and detailed TODO comments rather than hidden business logic
- [ ] Step 4: Run `cargo test -p core-domain` and verify service contract tests pass

### Task 8: Final simplification pass for workspace skeleton

**Files:**
- Modify: any files touched above that can be simplified without changing behavior
- Test: `cargo test -p core-domain`

- [ ] Step 1: Review the entire workspace domain skeleton against `docs/implement_rule.md` and `docs/implement_workspace_domain.md`
- [ ] Step 2: Remove avoidable duplication, flatten over-abstracted pieces, and simplify names/signatures where possible
- [ ] Step 3: Re-run `cargo test -p core-domain`
- [ ] Step 4: Record the final file list and notable simplifications in the task summary for reviewer subagents

---

Plan complete and saved to `docs/superpowers/plans/2026-03-22-workspace-domain-skeleton.md`. Ready to execute.
