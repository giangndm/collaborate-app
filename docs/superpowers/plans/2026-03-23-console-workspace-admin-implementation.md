# Console Workspace Admin Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Xây dựng `workspace-server` có auth/password session, persistence bằng SeaORM + SQLite, HTTP API đầy đủ cho workspace admin, và `frontends/console` dùng Refine để quản lý workspace end-to-end.

**Architecture:** Giữ `crates/core-domain` làm lõi typed domain; `services/workspace-server` là app hexagonal chứa config, auth, session store, SeaORM repositories, HTTP routes, và static embedding; `frontends/console` là admin SPA dùng Refine + Ant Design, gọi trực tiếp `/api/...` qua Vite proxy ở dev và embedded static ở release.

**Tech Stack:** Rust 2024, Axum, Tokio, SeaORM, SQLite, Clap, Serde, ThisError, Derive More, React, Vite, TypeScript, Refine, Ant Design.

**Spec Note:** Kế hoạch này bám theo spec đã được approve tại `docs/superpowers/specs/2026-03-23-console-workspace-admin-design.md`, trong đó đợt hiện tại chỉ implement `mock password auth`; `Clerk` là cải tiến tiếp theo và chưa nằm trong phạm vi thực thi của plan này.

**Package Note:** Về tên sản phẩm, backend là `workspace-server`; về crate/package hiện tại trong repo, service này vẫn đang mang tên `console`. Mọi lệnh `cargo test -p console` trong plan đang nhắm đúng service backend hiện hữu.

---

## File Structure

### Core domain

- Modify: `crates/core-domain/Cargo.toml`
- Modify: `crates/core-domain/src/lib.rs`
- Modify: `crates/core-domain/src/domain.rs`
- Modify: `crates/core-domain/src/domain/workspace/mod.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/membership.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/permissions.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/credentials.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/sync.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/errors.rs`
- Modify: `crates/core-domain/src/domain/workspace/entity/workspace.rs`
- Modify: `crates/core-domain/src/domain/workspace/entity/membership.rs`
- Modify: `crates/core-domain/src/domain/workspace/entity/user.rs`
- Modify: `crates/core-domain/src/domain/workspace/guards/workspace_member_guard.rs`
- Modify: `crates/core-domain/src/domain/workspace/guards/super_admin_guard.rs`
- Modify: `crates/core-domain/src/domain/workspace/guards/mod.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/workspace_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/membership_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/user_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/secret_store.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/mod.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/workspace_service.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/sync_service.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/mod.rs`

### Workspace server

- Modify: `Cargo.toml`
- Modify: `services/workspace-server/Cargo.toml`
- Replace: `services/workspace-server/src/main.rs`
- Create: `services/workspace-server/src/lib.rs`
- Create: `services/workspace-server/src/app/mod.rs`
- Create: `services/workspace-server/src/app/state.rs`
- Create: `services/workspace-server/src/config/mod.rs`
- Create: `services/workspace-server/src/config/cli.rs`
- Create: `services/workspace-server/src/config/settings.rs`
- Create: `services/workspace-server/src/auth/mod.rs`
- Create: `services/workspace-server/src/auth/actor.rs`
- Create: `services/workspace-server/src/auth/password.rs`
- Create: `services/workspace-server/src/auth/session.rs`
- Create: `services/workspace-server/src/auth/extractor.rs`
- Create: `services/workspace-server/src/persistence/mod.rs`
- Create: `services/workspace-server/src/persistence/db.rs`
- Create: `services/workspace-server/src/persistence/entities/mod.rs`
- Create: `services/workspace-server/src/persistence/entities/users.rs`
- Create: `services/workspace-server/src/persistence/entities/auth_sessions.rs`
- Create: `services/workspace-server/src/persistence/entities/workspaces.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_memberships.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_credentials.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_credential_secret_versions.rs`
- Create: `services/workspace-server/src/persistence/repositories/mod.rs`
- Create: `services/workspace-server/src/persistence/repositories/workspace_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/user_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/membership_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/secret_store.rs`
- Create: `services/workspace-server/src/persistence/seed.rs`
- Create: `services/workspace-server/src/http/mod.rs`
- Create: `services/workspace-server/src/http/router.rs`
- Create: `services/workspace-server/src/http/error.rs`
- Create: `services/workspace-server/src/http/dto/mod.rs`
- Create: `services/workspace-server/src/http/dto/auth.rs`
- Create: `services/workspace-server/src/http/dto/workspaces.rs`
- Create: `services/workspace-server/src/http/dto/members.rs`
- Create: `services/workspace-server/src/http/dto/credentials.rs`
- Create: `services/workspace-server/src/http/dto/sync.rs`
- Create: `services/workspace-server/src/http/handlers/mod.rs`
- Create: `services/workspace-server/src/http/handlers/auth.rs`
- Create: `services/workspace-server/src/http/handlers/workspaces.rs`
- Create: `services/workspace-server/src/http/handlers/members.rs`
- Create: `services/workspace-server/src/http/handlers/credentials.rs`
- Create: `services/workspace-server/src/http/handlers/sync.rs`
- Create: `services/workspace-server/src/http/handlers/health.rs`
- Create: `services/workspace-server/src/static_assets/mod.rs`

### Console frontend

- Modify: `frontends/console/package.json`
- Modify: `frontends/console/vite.config.ts`
- Modify: `frontends/console/tsconfig.app.json`
- Replace: `frontends/console/src/main.tsx`
- Replace: `frontends/console/src/App.tsx`
- Replace: `frontends/console/src/App.css`
- Replace: `frontends/console/src/index.css`
- Create: `frontends/console/src/app/providers.tsx`
- Create: `frontends/console/src/app/auth-provider.ts`
- Create: `frontends/console/src/app/data-provider.ts`
- Create: `frontends/console/src/app/api.ts`
- Create: `frontends/console/src/app/resources.tsx`
- Create: `frontends/console/src/pages/login.tsx`
- Create: `frontends/console/src/pages/dashboard.tsx`
- Create: `frontends/console/src/pages/workspaces/list.tsx`
- Create: `frontends/console/src/pages/workspaces/create.tsx`
- Create: `frontends/console/src/pages/workspaces/edit.tsx`
- Create: `frontends/console/src/pages/workspaces/show.tsx`
- Create: `frontends/console/src/pages/members/list.tsx`
- Create: `frontends/console/src/pages/credentials/list.tsx`
- Create: `frontends/console/src/components/layout.tsx`

### Release / verification

- Create: `.github/workflows/workspace-console-release.yml`
- Create: `services/workspace-server/build.rs` (nếu cần embed assets compile-time)
- Create: `services/workspace-server/tests/http_auth.rs`
- Create: `services/workspace-server/tests/http_workspaces.rs`
- Create: `services/workspace-server/tests/http_members.rs`
- Create: `services/workspace-server/tests/http_credentials.rs`
- Create: `services/workspace-server/tests/http_sync.rs`
- Create: `frontends/console/src/app/auth-provider.test.ts`
- Create: `frontends/console/src/app/data-provider.test.ts`

## Chunk 1: Core Domain + Auth + Persistence + HTTP

### Task 1: Simplify workspace roles and add missing permissions

**Files:**

- Modify: `crates/core-domain/src/domain/workspace/types/membership.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/permissions.rs`
- Modify: `crates/core-domain/src/domain/workspace/guards/workspace_member_guard.rs`
- Modify: `crates/core-domain/src/domain/workspace/guards/super_admin_guard.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add tests proving:

- `WorkspaceRole` chỉ còn `Owner` và `Member`
- `Member` chỉ derive được read permission
- `Owner` derive được read/write permission
- `IntegrationGuard` là minting surface cho `WorkspacesReadPermission`
- `SuperAdminGuard` vẫn chỉ derive workspace-scoped permission

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p core-domain workspace_roles`
Expected: FAIL vì enum/permission surface hiện tại vẫn còn `Admin`

- [ ] **Step 3: Write minimal implementation**

Xóa `Admin` khỏi workspace role enum, thêm `IntegrationGuard` cho trusted internal communication, giữ `SuperAdminGuard` actor-scoped, và cập nhật permission/guard test helpers.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p core-domain`
Expected: PASS cho phần workspace domain sau khi sửa test cũ liên quan `Admin`

- [ ] **Step 5: Commit**

```bash
git add crates/core-domain/src/domain/workspace
git commit -m "refactor: simplify workspace role permissions"
```

### Task 2: Add domain read models and member management use cases

**Files:**

- Modify: `crates/core-domain/src/domain/workspace/entity/workspace.rs`
- Modify: `crates/core-domain/src/domain/workspace/entity/membership.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/errors.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/sync.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/workspace_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/membership_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/user_repository.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/workspace_service.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add unit tests named:

- `non_super_admin_cannot_create_workspace_via_creator_guard`
- `create_workspace_bootstraps_first_owner`
- `list_workspaces_visible_to_actor_returns_all_for_super_admin`
- `list_workspaces_visible_to_actor_returns_only_memberships_for_member`
- `get_workspace_detail_returns_workspace_with_policy`
- `update_workspace_updates_default_room_policy`
- `list_members_returns_workspace_members`
- `add_member_returns_member_already_exists_for_duplicate`
- `remove_member_rejects_removing_last_owner`
- `owner_cannot_remove_another_owner`
- `change_member_role_updates_member_to_member_only`
- `change_member_role_rejects_last_owner_demotion`
- `only_super_admin_can_promote_member_to_owner`
- `owner_cannot_create_another_owner_via_role_change`
- `member_cannot_mutate_workspace`
- `workspace_mutations_bump_last_updated`

Các test trên phải bao phủ luôn `default_room_policy` như một phần của workspace aggregate, thay vì tạo bảng policy riêng trong đợt này.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p core-domain create_workspace_bootstraps_first_owner`
Expected: FAIL vì service/entity/port surface chưa đủ

- [ ] **Step 3: Write minimal implementation**

Thêm các use case đọc workspace, update workspace và `default_room_policy`, list membership, add/remove member, change member role, rule visibility, rejection path cho non-`SuperAdmin` ở `WorkspaceCreatorGuard`, và `last_updated` mutation trong domain.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p core-domain create_workspace_bootstraps_first_owner`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core-domain/src/domain/workspace/entity/workspace.rs crates/core-domain/src/domain/workspace/entity/membership.rs crates/core-domain/src/domain/workspace/types/errors.rs crates/core-domain/src/domain/workspace/types/sync.rs crates/core-domain/src/domain/workspace/ports/workspace_repository.rs crates/core-domain/src/domain/workspace/ports/membership_repository.rs crates/core-domain/src/domain/workspace/ports/user_repository.rs crates/core-domain/src/domain/workspace/service/workspace_service.rs crates/core-domain/src/domain/workspace/service/mod.rs
git commit -m "feat: add workspace member management use cases"
```

### Task 3: Add domain credential and sync use cases

**Files:**

- Modify: `crates/core-domain/src/domain/workspace/types/credentials.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/errors.rs`
- Modify: `crates/core-domain/src/domain/workspace/types/sync.rs`
- Modify: `crates/core-domain/src/domain/workspace/ports/secret_store.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/workspace_service.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/sync_service.rs`
- Modify: `crates/core-domain/src/domain/workspace/service/mod.rs`

- [ ] **Step 1: Write the failing tests**

Add unit tests named:

- `list_credentials_returns_metadata_only`
- `create_credential_returns_plaintext_once_and_stores_metadata`
- `rotate_secret_bumps_version_and_keeps_api_key_id`
- `member_cannot_create_credential`
- `member_cannot_rotate_secret`
- `owner_can_create_credential_and_rotate_secret`
- `credential_mutations_bump_last_updated`
- `sync_service_reads_with_workspaces_read_permission`
- `export_sync_payload_contains_policy_and_credential_verifiers`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p core-domain list_credentials_returns_metadata_only`
Expected: FAIL vì credential/sync contracts chưa đủ

- [ ] **Step 3: Write minimal implementation**

Thêm credential metadata model, rotation flow, sync payload đầy đủ, và permission path cho `WorkspacesReadPermission`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p core-domain list_credentials_returns_metadata_only`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core-domain/src/domain/workspace/types/credentials.rs crates/core-domain/src/domain/workspace/types/errors.rs crates/core-domain/src/domain/workspace/types/sync.rs crates/core-domain/src/domain/workspace/ports/secret_store.rs crates/core-domain/src/domain/workspace/service/workspace_service.rs crates/core-domain/src/domain/workspace/service/sync_service.rs crates/core-domain/src/domain/workspace/service/mod.rs
git commit -m "feat: add workspace credential and sync use cases"
```

### Task 4: Add workspace-server config and auth contracts

**Files:**

- Modify: `services/workspace-server/Cargo.toml`
- Replace: `services/workspace-server/src/main.rs`
- Create: `services/workspace-server/src/lib.rs`
- Create: `services/workspace-server/src/config/mod.rs`
- Create: `services/workspace-server/src/config/cli.rs`
- Create: `services/workspace-server/src/config/settings.rs`
- Create: `services/workspace-server/src/auth/mod.rs`
- Create: `services/workspace-server/src/auth/actor.rs`
- Create: `services/workspace-server/src/auth/password.rs`
- Create: `services/workspace-server/src/auth/session.rs`
- Create: `services/workspace-server/src/auth/extractor.rs`

- [ ] **Step 1: Write the failing tests**

Add unit tests named:

- `parse_mock_auth_accounts_from_cli`
- `parse_mock_auth_accounts_from_env`
- `parse_sync_token_from_cli_or_env`
- `parse_http_bind_address_from_cli_or_env`
- `parse_database_path_from_cli_or_env`
- `parse_static_asset_mode_from_cli_or_env`
- `parse_seed_mode_from_cli_or_env`
- `password_auth_rejects_invalid_credentials`
- `session_cookie_contract_is_stable`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console parse_mock_auth_accounts_from_cli`
Expected: FAIL vì config/auth modules chưa tồn tại

- [ ] **Step 3: Write minimal implementation**

Thêm `AppConfig` đầy đủ cho bind address, DB path/url, auth provider kind, static mock accounts, sync token, static asset mode, seed mode, auth value types, cookie/session contracts, và `AuthenticatedActor` có đủ `user_id`, `global_role`, `provider_subject/id`, `display_name`, `email`; chưa gắn DB-backed session store ở task này.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console parse_mock_auth_accounts_from_cli`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add services/workspace-server/Cargo.toml services/workspace-server/src/main.rs services/workspace-server/src/lib.rs services/workspace-server/src/config/mod.rs services/workspace-server/src/config/cli.rs services/workspace-server/src/config/settings.rs services/workspace-server/src/auth/mod.rs services/workspace-server/src/auth/actor.rs services/workspace-server/src/auth/password.rs services/workspace-server/src/auth/session.rs services/workspace-server/src/auth/extractor.rs
git commit -m "feat: add workspace server config and auth contracts"
```

### Task 5: Add migrations and persistence foundations

**Files:**

- Modify: `Cargo.toml`
- Create: `services/workspace-server/migration/Cargo.toml`
- Create: `services/workspace-server/migration/src/lib.rs`
- Create: `services/workspace-server/migration/src/main.rs`
- Create: `services/workspace-server/migration/src/m20260323_000001_create_users.rs`
- Create: `services/workspace-server/migration/src/m20260323_000002_create_auth_sessions.rs`
- Create: `services/workspace-server/migration/src/m20260323_000003_create_workspaces.rs`
- Create: `services/workspace-server/migration/src/m20260323_000004_create_workspace_memberships.rs`
- Create: `services/workspace-server/migration/src/m20260323_000005_create_workspace_credentials.rs`
- Create: `services/workspace-server/migration/src/m20260323_000006_create_workspace_credential_secret_versions.rs`
- Create: `services/workspace-server/src/persistence/mod.rs`
- Create: `services/workspace-server/src/persistence/db.rs`
- Create: `services/workspace-server/src/persistence/entities/mod.rs`
- Create: `services/workspace-server/src/persistence/entities/users.rs`
- Create: `services/workspace-server/src/persistence/entities/auth_sessions.rs`
- Create: `services/workspace-server/src/persistence/entities/workspaces.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_memberships.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_credentials.rs`
- Create: `services/workspace-server/src/persistence/entities/workspace_credential_secret_versions.rs`

- [ ] **Step 1: Write the failing tests**

Add persistence tests named:

- `bootstraps_sqlite_in_memory`
- `runs_migrations_on_empty_database`
- `persists_auth_session_and_supports_revocation`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console bootstraps_sqlite_in_memory`
Expected: FAIL vì migration/schema chưa tồn tại

- [ ] **Step 3: Write minimal implementation**

Tạo migration crate, SeaORM entities, DB bootstrap, và schema tối thiểu cho session/workspace data.

Schema `workspaces` trong task này phải chứa luôn các field cho `default_room_policy` để persistence bám đúng spec control-plane hiện tại mà không tách thêm module room policy riêng.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console bootstraps_sqlite_in_memory`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml services/workspace-server/migration services/workspace-server/src/persistence/mod.rs services/workspace-server/src/persistence/db.rs services/workspace-server/src/persistence/entities/mod.rs services/workspace-server/src/persistence/entities/users.rs services/workspace-server/src/persistence/entities/auth_sessions.rs services/workspace-server/src/persistence/entities/workspaces.rs services/workspace-server/src/persistence/entities/workspace_memberships.rs services/workspace-server/src/persistence/entities/workspace_credentials.rs services/workspace-server/src/persistence/entities/workspace_credential_secret_versions.rs
git commit -m "feat: add workspace server migrations and schema"
```

### Task 6: Add repositories, seeds, and member-candidate queries

**Files:**

- Create: `services/workspace-server/src/persistence/repositories/mod.rs`
- Create: `services/workspace-server/src/persistence/repositories/workspace_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/user_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/membership_repository.rs`
- Create: `services/workspace-server/src/persistence/repositories/secret_store.rs`
- Create: `services/workspace-server/src/persistence/repositories/auth_session_repository.rs`
- Create: `services/workspace-server/src/persistence/seed.rs`
- Modify: `services/workspace-server/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add persistence tests named:

- `seeds_default_users_and_workspaces`
- `member_candidate_query_excludes_existing_members`
- `member_candidate_query_applies_owner_prefix_rule`
- `workspace_repository_bumps_last_updated_transactionally`
- `session_repository_resolves_cookie_session_and_supports_logout_invalidation`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console seeds_default_users_and_workspaces`
Expected: FAIL vì repository/seed/query logic chưa tồn tại

- [ ] **Step 3: Write minimal implementation**

Thêm repository adapters map domain types, `auth_session_repository` cho DB-backed session store/resolution, seed baseline data, query hỗ trợ `member-candidates`, và helper bump `last_updated` transactionally.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console seeds_default_users_and_workspaces`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add services/workspace-server/src/lib.rs services/workspace-server/src/persistence/repositories/mod.rs services/workspace-server/src/persistence/repositories/workspace_repository.rs services/workspace-server/src/persistence/repositories/user_repository.rs services/workspace-server/src/persistence/repositories/membership_repository.rs services/workspace-server/src/persistence/repositories/secret_store.rs services/workspace-server/src/persistence/repositories/auth_session_repository.rs services/workspace-server/src/persistence/seed.rs
git commit -m "feat: add workspace server repositories and seeds"
```

### Task 7: Add auth and health HTTP endpoints

**Files:**

- Create: `services/workspace-server/src/app/mod.rs`
- Create: `services/workspace-server/src/app/state.rs`
- Create: `services/workspace-server/src/http/mod.rs`
- Create: `services/workspace-server/src/http/router.rs`
- Create: `services/workspace-server/src/http/error.rs`
- Create: `services/workspace-server/src/http/dto/mod.rs`
- Create: `services/workspace-server/src/http/dto/auth.rs`
- Create: `services/workspace-server/src/http/handlers/mod.rs`
- Create: `services/workspace-server/src/http/handlers/auth.rs`
- Create: `services/workspace-server/src/http/handlers/health.rs`
- Create: `services/workspace-server/tests/http_auth.rs`

- [ ] **Step 1: Write the failing tests**

Add integration tests named:

- `http_auth_login_sets_session_cookie`
- `http_auth_login_returns_invalid_credentials`
- `http_auth_login_returns_normalized_actor_payload`
- `http_auth_session_requires_login`
- `http_auth_session_unauthenticated_uses_error_envelope`
- `http_auth_session_returns_normalized_actor_payload`
- `http_auth_logout_revokes_session`
- `http_auth_logout_clears_cookie`
- `http_auth_logout_returns_204`
- `http_auth_cookie_uses_workspace_console_session_name`
- `http_auth_cookie_uses_http_only_same_site_lax_and_ttl`
- `http_health_returns_ok`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console --test http_auth http_auth_login_sets_session_cookie`
Expected: FAIL vì router/handlers chưa tồn tại

- [ ] **Step 3: Write minimal implementation**

Tạo app state, router nền, auth/health DTOs, error mapping, và handlers cho login/logout/session/health.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console --test http_auth`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add services/workspace-server/src/app/mod.rs services/workspace-server/src/app/state.rs services/workspace-server/src/http/mod.rs services/workspace-server/src/http/router.rs services/workspace-server/src/http/error.rs services/workspace-server/src/http/dto/mod.rs services/workspace-server/src/http/dto/auth.rs services/workspace-server/src/http/handlers/mod.rs services/workspace-server/src/http/handlers/auth.rs services/workspace-server/src/http/handlers/health.rs services/workspace-server/tests/http_auth.rs
git commit -m "feat: add workspace server auth endpoints"
```

### Task 8: Add workspace, members, credentials, and sync HTTP endpoints

**Files:**

- Create: `services/workspace-server/src/http/dto/workspaces.rs`
- Create: `services/workspace-server/src/http/dto/members.rs`
- Create: `services/workspace-server/src/http/dto/credentials.rs`
- Create: `services/workspace-server/src/http/dto/sync.rs`
- Create: `services/workspace-server/src/http/handlers/workspaces.rs`
- Create: `services/workspace-server/src/http/handlers/members.rs`
- Create: `services/workspace-server/src/http/handlers/credentials.rs`
- Create: `services/workspace-server/src/http/handlers/sync.rs`
- Modify: `services/workspace-server/src/http/router.rs`
- Test: `services/workspace-server/tests/http_workspaces.rs`
- Test: `services/workspace-server/tests/http_members.rs`
- Test: `services/workspace-server/tests/http_credentials.rs`
- Test: `services/workspace-server/tests/http_sync.rs`

- [ ] **Step 1: Write the failing tests**

Add integration tests named:

- `http_workspaces_list_uses_refine_envelope`
- `http_workspaces_list_supports_page_per_page_sort_order_filter`
- `http_workspaces_create_bootstraps_owner_membership`
- `http_workspaces_create_returns_detail_envelope`
- `http_workspaces_show_returns_detail_envelope`
- `http_workspaces_patch_updates_policy`
- `http_workspaces_patch_returns_detail_envelope`
- `http_members_list_returns_workspace_members`
- `http_members_candidates_enforce_owner_query_rules`
- `http_members_candidates_return_active_non_members_only`
- `http_members_candidates_super_admin_uses_case_insensitive_contains_search`
- `http_members_candidates_owner_query_too_short_returns_400_query_too_short`
- `http_member_candidates_list_uses_standard_envelope`
- `http_members_add_returns_member_already_exists`
- `http_members_patch_updates_role_with_owner_rules`
- `http_members_delete_returns_204`
- `http_credentials_list_uses_standard_envelope`
- `http_credentials_list_returns_metadata_only`
- `http_credentials_create_requires_label`
- `http_credentials_member_forbidden`
- `http_credentials_owner_allowed`
- `http_credentials_rotate_returns_plaintext_once`
- `http_sync_change_feed_returns_stable_cursor_order`
- `http_sync_change_feed_without_cursor_returns_first_page_ordered_by_last_updated_then_workspace_id`
- `http_sync_requires_valid_bearer_token`
- `http_sync_missing_workspace_returns_404`
- `http_sync_change_feed_returns_limit_has_more_and_next_cursor`
- `http_sync_empty_page_preserves_cursor_contract`
- `http_sync_detail_returns_workspace_payload`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console --test http_workspaces http_workspaces_list_uses_refine_envelope`
Expected: FAIL vì resource handlers/DTO chưa đủ

- [ ] **Step 3: Write minimal implementation**

Tạo DTO conversions, typed extractors, resource handlers, sync cursor handling, và router wiring cho tất cả admin/sync endpoints còn lại.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console --test http_workspaces && cargo test -p console --test http_members && cargo test -p console --test http_credentials && cargo test -p console --test http_sync`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add services/workspace-server/src/http/dto/workspaces.rs services/workspace-server/src/http/dto/members.rs services/workspace-server/src/http/dto/credentials.rs services/workspace-server/src/http/dto/sync.rs services/workspace-server/src/http/handlers/workspaces.rs services/workspace-server/src/http/handlers/members.rs services/workspace-server/src/http/handlers/credentials.rs services/workspace-server/src/http/handlers/sync.rs services/workspace-server/src/http/router.rs services/workspace-server/tests/http_workspaces.rs services/workspace-server/tests/http_members.rs services/workspace-server/tests/http_credentials.rs services/workspace-server/tests/http_sync.rs
git commit -m "feat: add workspace server resource endpoints"
```

## Chunk 2: Frontend + Embedded Release + Verification

### Task 6: Replace Vite starter with Refine app shell

**Files:**

- Modify: `frontends/console/package.json`
- Modify: `frontends/console/vite.config.ts`
- Replace: `frontends/console/src/main.tsx`
- Replace: `frontends/console/src/App.tsx`
- Replace: `frontends/console/src/App.css`
- Replace: `frontends/console/src/index.css`
- Create: `frontends/console/src/app/providers.tsx`
- Create: `frontends/console/src/app/resources.tsx`
- Create: `frontends/console/src/components/layout.tsx`

- [ ] **Step 1: Write the failing tests**

Add frontend tests verifying:

- app boots with Refine + Antd
- unauthenticated users are redirected to sign-in
- authenticated shell renders left menu resources
- `Create workspace` action chỉ hiện với `SuperAdmin`
- frontend API client keeps using relative `/api/...` paths
- Vite config proxies `/api` to backend in dev

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --runInBand` or project-equivalent frontend test command
Expected: FAIL vì Refine app shell chưa tồn tại

- [ ] **Step 3: Write minimal implementation**

Thay starter app bằng Refine shell, router, auth gate, layout và resource registration.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- --runInBand`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add frontends/console/package.json frontends/console/vite.config.ts frontends/console/src/main.tsx frontends/console/src/App.tsx frontends/console/src/App.css frontends/console/src/index.css frontends/console/src/app/providers.tsx frontends/console/src/app/resources.tsx frontends/console/src/components/layout.tsx
git commit -m "feat: add console refine app shell"
```

### Task 7: Add auth provider and API client

**Files:**

- Create: `frontends/console/src/app/api.ts`
- Create: `frontends/console/src/app/auth-provider.ts`
- Create: `frontends/console/src/app/data-provider.ts`
- Create: `frontends/console/src/pages/login.tsx`
- Test: `frontends/console/src/app/auth-provider.test.ts`
- Test: `frontends/console/src/app/data-provider.test.ts`

- [ ] **Step 1: Write the failing tests**

Add tests for:

- login request body / logout / session check
- 401 handling
- CRUD envelope mapping
- sync endpoint exemption from CRUD envelope assumptions
- all requests stay on relative `/api/...` paths
- password sign-in form render/submit/error/success behavior

- [ ] **Step 2: Run test to verify it fails**

Run: frontend test command for auth/data provider tests
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

Xây API wrapper, Refine auth provider dùng password session, data provider cho list/detail/create/update/delete/custom endpoints.

- [ ] **Step 4: Run test to verify it passes**

Run: frontend test command
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add frontends/console/src/app frontends/console/src/pages/login.tsx
git commit -m "feat: add console auth and api providers"
```

### Task 8: Add workspace, members, and credentials pages

**Files:**

- Create: `frontends/console/src/pages/dashboard.tsx`
- Create: `frontends/console/src/pages/workspaces/list.tsx`
- Create: `frontends/console/src/pages/workspaces/create.tsx`
- Create: `frontends/console/src/pages/workspaces/edit.tsx`
- Create: `frontends/console/src/pages/workspaces/show.tsx`
- Create: `frontends/console/src/pages/members/list.tsx`
- Create: `frontends/console/src/pages/credentials/list.tsx`

- [ ] **Step 1: Write the failing tests**

Add tests for:

- dashboard renders summary cards/shortcuts
- workspace list shows `workspace_id`, `name`, `slug`, `status`, `last_updated`
- workspace create form covers `name`, `slug`, `status`, `default_room_policy.guest_join_enabled`, `default_room_policy.token_ttl_seconds`
- workspace edit form covers `name`, `status`, `default_room_policy.guest_join_enabled`, `default_room_policy.token_ttl_seconds`
- workspace show page renders detail fields and policy values from the approved DTO contract
- members list page renders `user_id`, `email`, `display_name`, `workspace_role`, `user_status`
- member add/update/remove flows
- member candidate search rules for `SuperAdmin` contains vs `Owner` email prefix
- `query_too_short` handling for owner search
- active/non-member-only candidate results
- only `SuperAdmin` can promote to owner
- `Owner` cannot remove another `Owner`
- cannot remove or demote the last owner
- credentials list page renders `api_key_id`, `label`, `status`, `version`, `created_at`, `rotated_at`
- credential create/rotate flow including one-time plaintext secret display
- action visibility matrix for `SuperAdmin`, `Owner`, and `Member` across workspace/member/credential actions

- [ ] **Step 2: Run test to verify it fails**

Run: frontend page test command
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

Tận dụng Refine list/create/edit/show components, xây dashboard đơn giản với summary cards/quick links, và chỉ custom các action như add member, rotate secret, member candidate search, one-time secret reveal.

- [ ] **Step 4: Run test to verify it passes**

Run: frontend test command
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add frontends/console/src/pages frontends/console/src/app/resources.tsx
git commit -m "feat: add console admin resource pages"
```

### Task 9: Add static embedding and release workflow

**Files:**

- Create: `.github/workflows/workspace-console-release.yml`
- Create or Modify: `services/workspace-server/build.rs`
- Create or Modify: `services/workspace-server/src/static_assets/mod.rs`
- Modify: `services/workspace-server/src/http/router.rs`

- [ ] **Step 1: Write the failing tests**

Add tests for:

- backend serves embedded `index.html`
- backend serves built static assets
- non-API SPA routes fall back to embedded `index.html`
- `/api/health` still works when static serving enabled

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p console --lib static_assets`
Expected: FAIL

- [ ] **Step 3: Write minimal implementation**

Embed built frontend assets, add static fallback route, and GitHub Actions workflow that builds frontend before Rust release build.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p console static_assets`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/workspace-console-release.yml services/workspace-server/build.rs services/workspace-server/src/static_assets services/workspace-server/src/http/router.rs
git commit -m "feat: embed console frontend in workspace server"
```

### Task 10: Full verification and browser check

**Files:**

- Modify: any touched files from earlier tasks only if verification uncovers real defects

- [ ] **Step 1: Run backend verification**

Run: `cargo fmt --all --check && cargo test && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS or only pre-existing approved warnings outside changed scope if unavoidable

- [ ] **Step 2: Run frontend verification**

Run: frontend lint/test/build commands in `frontends/console`
Expected: PASS

- [ ] **Step 3: Run release-like build verification**

Run: frontend build then Rust test/build path that exercises embedded assets
Expected: PASS

- [ ] **Step 4: Run Chrome MCP verification**

Verify these flows against local app with mock auth:

- login with `superadmin`
- create workspace
- edit workspace policy
- browse members
- search and add member
- update member role to `member` or `owner` according to permission matrix
- create credential
- rotate secret

- [ ] **Step 5: Commit verification fixes if needed**

```bash
git add <only files changed by verification fixes>
git commit -m "fix: address verification issues"
```
