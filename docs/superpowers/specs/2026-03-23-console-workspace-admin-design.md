# Thiết kế Console Admin và Workspace Server

## 1. Mục tiêu

Tài liệu này chốt thiết kế end-to-end cho `workspace-server` và `console frontend`
để tạo ra một control plane hoàn chỉnh cho quản lý workspace.

Phạm vi bao gồm:

- admin console nhiều trang theo phong cách `refine`
- `workspace-server` có HTTP API và wiring đầy đủ vào `core-domain`
- repository dùng `SeaORM` theo hướng type-safe cho dữ liệu workspace
- auth abstraction hỗ trợ cả `Clerk` và `mock password auth`
- bản release embed static build của `console frontend` vào `workspace-server`
- test đầy đủ và browser verification bằng Chrome MCP cho các luồng quản trị chính

Tài liệu này ưu tiên các nguyên tắc sau:

- kiến trúc hexagonal cho Rust backend
- compile-time safety và typed domain boundary
- giảm duplication giữa domain, repo, HTTP và frontend data flow
- DX tốt cho local development, test và review

## 2. Kết quả cần đạt

Sau khi hoàn thành, repo cần có các tính chất sau:

- admin có thể đăng nhập bằng `Clerk` hoặc `password login` tùy theo cấu hình
- giao diện `console` dùng `refine` cho left menu, list, paging, detail, edit và
  create
- frontend luôn gọi API bằng đường dẫn tương đối `/api/...`
- `workspace-server` cung cấp đầy đủ các use case cho workspace, member,
  credential, secret rotation và sync export
- `SeaORM` là adapter persistence chính; `dev` dùng SQLite on-disk, `test` dùng
  SQLite in-memory
- bản release có thể phục vụ static console asset trực tiếp từ `workspace-server`

## 3. Kiến trúc tổng thể

```text
frontends/console
  -> refine resources + auth provider + data provider
  -> /api/*
  -> workspace-server
       -> HTTP handlers
       -> auth session resolution
       -> guard/permission derivation
       -> core-domain workspace services
       -> SeaORM repositories
       -> SQLite
```

Boundary được chốt như sau:

- `crates/core-domain`: giữ typed domain và use case orchestration, không phụ
  thuộc framework hay storage
- `services/workspace-server`: app layer và adapter layer cho HTTP, auth,
  persistence, config và static asset serving
- `frontends/console`: admin SPA dùng `refine`, tận dụng tối đa những phần có sẵn
  để giảm code custom

## 4. Thiết kế backend

### 4.1 Workspace domain responsibilities

`workspace domain` tiếp tục là control-plane core domain và được mở rộng từ
skeleton hiện tại thành service đầy đủ cho các use case sau:

- `create_workspace`
- `list_workspaces_visible_to_actor`
- `get_workspace_detail`
- `update_workspace`
- `list_members`
- `change_member_role`
- `list_credentials`
- `create_credential`
- `rotate_secret`
- `export_sync_payload`

Luồng authz vẫn theo cấu trúc:

```text
HTTP request
  -> authenticated actor context
  -> guard
  -> WorkspaceCreatorGuard / WorkspaceReadPermission / WorkspaceWritePermission
  -> WorkspaceService / WorkspaceSyncService
```

Service signatures tiếp tục nhận typed guard hoặc permission thay vì raw role
string.

### 4.2 Rule bootstrap workspace

Rule bootstrap được chốt như sau:

- caller hợp lệ của `create_workspace` phải đi qua `WorkspaceCreatorGuard`
- trong đợt này, chỉ `SuperAdmin` mới có thể lấy được `WorkspaceCreatorGuard`
- ngay sau khi tạo workspace, chính actor tạo workspace trở thành membership
  `Owner` đầu tiên của workspace đó
- invariant này phải được persist cùng transaction với workspace creation để các
  use case `list_members` và `change_member_role` có baseline owner hợp lệ ngay từ
  đầu

Điều này cũng chốt luôn hành vi UI:

- nút `Create workspace` chỉ hiển thị cho actor có `global_role = SuperAdmin`
- seed test cần có ít nhất một mock account `SuperAdmin` để cover full create flow

### 4.3 Auth abstraction

Backend giới thiệu một auth boundary mới để `workspace-server` không bị khóa chặt
vào `Clerk`.

Các contract và type chính:

- `AuthProviderKind`: parse config và điều khiển wiring runtime
- `AuthenticatedActor`: actor context chuẩn hóa cho toàn bộ app
- `AuthSessionResolver`: resolve actor từ request/session/token
- `PasswordAuthenticator`: contract riêng cho mock password flow nếu cần tách
  biệt

`AuthenticatedActor` tối thiểu phải mang:

- `user_id`
- `global_role`
- provider subject/id để trace
- thông tin hiển thị cơ bản phục vụ UI và audit

Hai implementation baseline:

1. `Clerk`
   - frontend đăng nhập qua Clerk SDK/UI
   - mỗi request `/api/*` gửi `Authorization: Bearer <clerk_token>`
   - backend verify bearer token và map sang `AuthenticatedActor`
2. `Mock password auth`
   - frontend submit username/password tới backend
   - backend tạo session cookie `HttpOnly`
   - backend đọc cookie và map sang `AuthenticatedActor`

Lựa chọn provider do `clap` arg/env quyết định. Frontend có thể bật hoặc tắt
`Clerk login` và `password login` bằng env để tránh hiển thị UI không hợp lệ cho
runtime hiện tại.

### 4.4 Chuẩn hóa session API

Để frontend refine auth provider có contract ổn định, session API được chốt như
sau:

- `GET /api/auth/session`
  - trong `Clerk mode`: đọc bearer token hiện tại, verify và trả actor normalized
  - trong `mock password mode`: đọc cookie session và trả actor normalized
- `POST /api/auth/logout`
  - trong `Clerk mode`: trả `204`, frontend tự sign-out khỏi Clerk
  - trong `mock password mode`: xóa session cookie và trả `204`
- `POST /api/auth/password/login`
  - chỉ khả dụng khi bật `mock password auth`
  - nhận username/password, tạo session cookie và trả actor normalized

Như vậy frontend luôn dùng cùng một `session-check flow`, chỉ khác transport auth
ở tầng hạ tầng.

### 4.5 Persistence với SeaORM

`SeaORM` là adapter chính cho `workspace-server` vì cần DB-agnostic và query
surface có type rõ ràng. Bản đầu dùng SQLite để giảm complexity.

Config DB:

- `dev`: SQLite file on disk
- `test`: SQLite in-memory

Schema tối thiểu:

- `users`
- `auth_identities`
- `workspaces`
- `workspace_memberships`
- `workspace_credentials`
- `workspace_secret_versions`
- `mock_auth_accounts`

Nguyên tắc adapter:

- SeaORM entity/model chỉ sống trong adapter layer
- repository adapter map sang typed domain entities/value objects
- migrations được chạy từ `workspace-server` startup hoặc test bootstrap theo
  config

`mock_auth_accounts` được chốt là bảng riêng mapping 1:1 tới `users` qua `user_id`.
Nó chứa username/password digest cho mock login mà không làm bẩn domain model của
user.

`auth_identities` là bảng ánh xạ generic giữa external identity và local user:

- `provider_kind` như `clerk`
- `provider_subject`
- `user_id`

Trong `Clerk mode`, flow provisioning được chốt như sau:

- backend verify Clerk token và lấy `provider_subject`, email, display name
- backend tìm `auth_identities(provider_kind = clerk, provider_subject = ... )`
- nếu đã có mapping, dùng `user_id` hiện có
- nếu chưa có mapping, backend tạo mới `users` + `auth_identities` trong cùng
  transaction
- `global_role` của user mới mặc định là `Member`, trừ khi match danh sách
  bootstrap/superadmin từ config seed

Nhờ vậy `create_workspace` auto-owner membership luôn có local `user_id` ổn định,
bất kể actor đến từ Clerk hay mock auth.

### 4.6 Mô hình credential và secret lifecycle

Trong phạm vi tài liệu này, `rotate_secret` không đụng tới system-wide fixed secret
mà `gateway` dùng để verify room access token theo `docs/system_spec.md`.

Ở đây, credential lifecycle được chốt như sau:

- `create_credential`
  - backend sinh `api_key_id` và plaintext `api_secret` mới
  - plaintext secret chỉ được trả về đúng một lần trong response tạo credential
  - persistence layer chỉ lưu secret digest/hash và metadata version
- `rotate_secret`
  - áp dụng cho một credential cụ thể, không phải cho toàn workspace
  - giữ nguyên `api_key_id`
  - sinh plaintext `api_secret` mới và tạo secret version mới
  - plaintext secret mới cũng chỉ được trả về đúng một lần trong response rotate
- `list_credentials`
  - chỉ trả metadata như `api_key_id`, `status`, `version`, `created_at`,
    `rotated_at`
  - không bao giờ trả lại plaintext secret sau lần create/rotate đầu tiên

Điều này dẫn tới API phải có định danh credential rõ ràng khi rotate secret.

### 4.7 Đồng bộ `last_updated`

Để incremental sync triển khai được nhất quán, source of truth cho `last_updated`
được chốt là cột `last_updated` nằm trên bảng `workspaces`.

Rule cập nhật:

- mọi mutation workspace-scoped đều phải bump `workspaces.last_updated` trong cùng
  transaction
- các thay đổi cần bump bao gồm:
  - update workspace metadata/status
  - thay đổi membership
  - tạo credential
  - rotate secret
- nhờ đó, gateway chỉ cần theo dõi một cursor cấp workspace thay vì tự suy luận từ
  nhiều bảng con

Để tránh miss hoặc duplicate khi nhiều workspace có cùng timestamp, thứ tự sync
được chốt là:

- primary order: `last_updated ASC`
- secondary order: `workspace_id ASC`

Cursor incremental sync là cặp:

- `updated_after`
- `after_workspace_id`

Caller luôn tiếp tục từ item cuối cùng đã xử lý thành công theo đúng cặp cursor này.

### 4.8 HTTP API

`workspace-server` cung cấp JSON REST API dưới `/api`.

Nhóm endpoint dự kiến:

- auth
  - `POST /api/auth/password/login`
  - `POST /api/auth/logout`
  - `GET /api/auth/session`
- public
  - `GET /api/public/runtime-config`
- workspaces
  - `GET /api/workspaces`
  - `POST /api/workspaces`
  - `GET /api/workspaces/:workspace_id`
  - `PATCH /api/workspaces/:workspace_id`
- members
  - `GET /api/workspaces/:workspace_id/members`
  - `PATCH /api/workspaces/:workspace_id/members/:user_id`
- credentials
  - `GET /api/workspaces/:workspace_id/credentials`
  - `POST /api/workspaces/:workspace_id/credentials`
  - `POST /api/workspaces/:workspace_id/credentials/:api_key_id/rotate-secret`
- sync
  - `GET /api/sync/workspaces?updated_after=<rfc3339>&after_workspace_id=<id>&limit=<n>`
  - `GET /api/sync/workspaces/:workspace_id`
- health/system
  - `GET /api/health`

Handlers phải mỏng:

- parse và validate request DTO
- resolve actor
- tạo guard/permission
- gọi domain service
- map domain result sang HTTP response DTO

### 4.9 Contract list/detail cho Refine

Để `refine data provider` được pin sớm và không phải đoán response envelope, API
JSON được chốt như sau:

- list endpoints nhận các query params:
  - `page`
  - `per_page`
  - `sort`
  - `order`
  - `filter` cho text filter đơn giản nếu cần
- list response shape:

```json
{
  "data": [],
  "total": 0,
  "page": 1,
  "per_page": 20
}
```

- detail/create/update response shape:

```json
{
  "data": {}
}
```

- error response shape:

```json
{
  "error": {
    "code": "workspace_permission_mismatch",
    "message": "Thông điệp dễ hiểu cho người dùng hoặc lập trình viên"
  }
}
```

### 4.10 DTO shape tối thiểu

Để implementation plan có thể chốt schema, form và test contract, các payload tối
thiểu được pin như sau.

`POST /api/workspaces` request:

```json
{
  "name": "Acme Workspace",
  "slug": "acme-workspace",
  "status": "active",
  "default_room_policy": {
    "guest_join_enabled": false,
    "token_ttl_seconds": 3600
  }
}
```

`PATCH /api/workspaces/:workspace_id` request:

```json
{
  "name": "Acme Workspace",
  "status": "active",
  "default_room_policy": {
    "guest_join_enabled": true,
    "token_ttl_seconds": 1800
  }
}
```

`GET /api/auth/session` response:

```json
{
  "data": {
    "user_id": "usr_123",
    "display_name": "Demo Super Admin",
    "email": "superadmin@example.com",
    "global_role": "super_admin",
    "auth_provider": "mock_password"
  }
}
```

`GET /api/public/runtime-config` response:

```json
{
  "data": {
    "clerk_login_enabled": true,
    "password_login_enabled": true
  }
}
```

`member` DTO:

```json
{
  "user_id": "usr_123",
  "email": "owner@example.com",
  "display_name": "Workspace Owner",
  "workspace_role": "owner",
  "user_status": "active"
}
```

`POST /api/workspaces/:workspace_id/credentials` response:

```json
{
  "data": {
    "api_key_id": "key_123",
    "api_secret": "plain_secret_visible_once",
    "status": "active",
    "version": 1
  }
}
```

`POST /api/workspaces/:workspace_id/credentials/:api_key_id/rotate-secret` response:

```json
{
  "data": {
    "api_key_id": "key_123",
    "api_secret": "new_plain_secret_visible_once",
    "status": "active",
    "version": 2
  }
}
```

`GET /api/sync/workspaces/:workspace_id` response:

```json
{
  "data": {
    "workspace_id": "ws_123",
    "status": "active",
    "last_updated": "2026-03-23T10:00:00Z",
    "default_room_policy": {
      "guest_join_enabled": false,
      "token_ttl_seconds": 3600
    },
    "credential_verifiers": [
      {
        "api_key_id": "key_123",
        "version": 2,
        "status": "active",
        "verifier": {
          "algorithm": "argon2id",
          "digest": "...",
          "salt": "..."
        }
      }
    ]
  }
}
```

`credential_verifiers` là vật liệu verify đủ để `gateway` xác thực `workspace API
key / secret` ở local mà không cần gọi nóng về `Console`, nhưng vẫn không chứa raw
plaintext secret.

## 5. Thiết kế sync export

### 5.1 Sync auth contract

Sync export không dùng admin session auth. Thay vào đó, nó dùng machine-to-machine
auth riêng.

Contract được chốt như sau:

- sync endpoints dùng shared sync bearer token cấu hình bằng `clap`/env, ví dụ
  `Authorization: Bearer <workspace_sync_token>`
- token này dành cho `gateway` pull dữ liệu từ `workspace-server`
- baseline hiện tại: token toàn cục được quyền đọc tất cả syncable workspaces
- request sai token trả `401`
- workspace không tồn tại trả `404`
- hardening như per-workspace token hoặc allowlist để sau, không nằm trong đợt này

### 5.2 Sync guard boundary

Để không làm mờ typed permission direction của domain:

- global sync token được xử lý ở app layer bằng một guard riêng, ví dụ
  `SyncPullGuard`
- `SyncPullGuard` không thay thế auth model của admin session
- `SyncPullGuard` được phép tạo trực tiếp một `WorkspaceReadPermission` hạ tầng cho
  `workspace_id` mục tiêu mà không cần dựng fake admin actor
- rule này chỉ áp dụng cho sync export path; admin HTTP path vẫn phải đi qua
  actor-based guard thông thường
- endpoint liệt kê incremental changes là concern của app layer; nó không cần đẩy
  xuống domain service theo cùng bề mặt với admin use case

### 5.3 Incremental sync contract

Để phù hợp với hướng `Console -> Gateway periodic pull + incremental sync` trong
`docs/system_spec.md`, contract incremental sync được chốt như sau:

- `GET /api/sync/workspaces?updated_after=<rfc3339>&after_workspace_id=<id>&limit=<n>`
  - trả về danh sách workspace đã thay đổi sau mốc `updated_after`
  - mỗi item gồm `workspace_id` và `last_updated`
  - dùng để `gateway` phát hiện workspace nào cần pull lại
- `GET /api/sync/workspaces/:workspace_id`
  - trả về `WorkspaceSyncPayload` đầy đủ cho một workspace
- nếu `updated_after` vắng mặt, backend trả page đầu tiên theo thứ tự
  `last_updated ASC`
- cursor baseline là cặp `last_updated + workspace_id`; caller tiếp tục lần sau
  bằng item cuối cùng đã xử lý thành công

Sync payload trong đợt này phải chứa đủ dữ liệu để `gateway` tự verify
`workspace API key / secret` tại local, tối thiểu gồm:

- `workspace_id`
- `status`
- `last_updated`
- `default_room_policy`
- `credential_verifiers`

Không cần CRUD riêng cho room catalog trong đợt này, nhưng `default_room_policy`
được quản lý như một phần của workspace edit để vẫn bám hướng tài liệu hệ thống về
room policy/control-plane metadata.

## 6. Thiết kế frontend

### 6.1 Hướng tiếp cận

`frontends/console` sẽ được chuyển thành admin app dùng `refine` thay vì tự build
UI shell. Mục tiêu là dùng lại nhiều nhất có thể các phần có sẵn của `refine` để
giảm code custom.

Thành phần có sẵn cần ưu tiên dùng:

- left menu
- resource routing
- list page
- paging/table
- create/edit/show pages
- form scaffolding và action button pattern

Chỉ custom ở các điểm cần thiết:

- trang `sign in`
- auth provider bridge giữa `Clerk` và `password login`
- action domain-specific như `rotate secret`

### 6.2 Stack refine được chốt

Để worker có thể lập kế hoạch và code theo một stack thống nhất, frontend stack
được chốt như sau:

- `@refinedev/core`
- `@refinedev/react-router`
- `@refinedev/antd`
- `react-router`
- `antd`

Lý do chọn `antd` thay vì headless/custom UI:

- refine hỗ trợ rất đầy đủ cho list/create/edit/show pattern
- left menu, table, paging và form layout đã sẵn có
- giảm đáng kể số lượng component phải tự viết trong đợt này

### 6.3 Screen set

Màn hình baseline:

- `Sign in`
- `Dashboard`
- `Workspaces / List`
- `Workspaces / Create`
- `Workspaces / Show/Edit`
- `Members / List + Update role`
- `Credentials / List + Create + Rotate secret`

Phong cách UI:

- nhiều trang, dễ đọc, rõ ràng, ưu tiên admin productivity
- không theo hướng marketing landing page
- gần với refine default admin shell hơn là một theme custom phức tạp

### 6.4 Data và auth trên frontend

Frontend sử dụng:

- refine `data provider` nối vào `/api/...`
- refine `auth provider` bọc quanh auth mode đang bật
- server runtime config để biết auth methods nào đang bật

Nguồn sự thật cho auth mode được chốt như sau:

- `workspace-server` là nguồn sự thật ở runtime
- frontend đọc `GET /api/public/runtime-config` khi khởi động để biết:
  - có bật `clerk_login` hay không
  - có bật `password_login` hay không
- frontend env chỉ được dùng như fallback trong local dev, không phải contract
  chính của bản release embed

Trong `dev`, Vite proxy `/api` sang `workspace-server` để frontend code luôn dùng
path tương đối `/api/...`.

## 7. Release flow và static embedding

Trong local dev:

- `frontends/console` chạy Vite riêng
- Vite proxy `/api` sang `workspace-server`
- `workspace-server` không cần reverse proxy frontend asset

Trong release:

- GitHub Actions build `frontends/console`
- artifact build được embed vào `workspace-server`
- server phục vụ static files và SPA fallback `index.html`
- `/api/*` vẫn được xử lý nội bộ bởi backend routes

Điều này giữ được hai tính chất:

- frontend code luôn gọi đường dẫn tương đối `/api/...`
- release deployment chỉ cần một binary/service cho control plane

## 8. Config và wiring

`workspace-server` dùng `clap` cho args + env.

`AppConfig` typed cần bao gồm ít nhất:

- HTTP bind address
- database path/url
- auth provider kind
- mock auth settings
- Clerk settings
- sync token settings
- static asset mode
- migration/seed mode

Seed baseline cho `dev/test`:

- 2 mock account
  - `superadmin`
  - `member-owner-demo`
- 1-2 workspace demo
- membership đủ để cover list/detail/member/credential flow

Nếu `mock auth` được bật, config có thể set username/password mặc định qua arg/env
để dễ khởi động local.

## 9. Testing strategy

Tất cả implementation cần follow TDD theo từng lát chức năng.

Mức test cần có:

1. Domain tests
   - verify service behavior, permission mismatch, role change invariants,
     credential/secret flow và sync export
2. Repository tests
   - SeaORM + SQLite in-memory
   - verify mapping giữa DB và domain types
3. HTTP integration tests
   - wiring thật với auth adapter, repo adapter, migrations và seed
   - verify session, workspace CRUD, member update, credential flow và sync auth
4. Frontend tests
   - auth screen behavior theo env flag
   - resource pages render/submit đúng với refine provider
5. Browser verification
   - login
   - create workspace
   - edit workspace
   - browse members
   - create credential
   - rotate secret

## 10. Verification và quality gates

Cần có command verify tối thiểu cho backend và frontend:

- `cargo fmt --all --check`
- `cargo test`
- `cargo clippy --workspace --all-targets -- -D warnings` nếu khả thi trong repo
- frontend lint/test/build command cho `frontends/console`
- release-like build để verify frontend embedding route

Chrome MCP verification sẽ chạy với `mock auth` để đảm bảo e2e admin flow có thể
lặp lại và không phụ thuộc external Clerk service.

## 11. File/module structure dự kiến

Hướng tách file cấp cao:

- `crates/core-domain/src/domain/workspace/...`
  - mở rộng ports/types/service để support use case đầy đủ
- `services/workspace-server/src/config/...`
  - clap parsing + typed config
- `services/workspace-server/src/http/...`
  - router, request/response DTO, handlers
- `services/workspace-server/src/auth/...`
  - auth abstraction, Clerk adapter, mock adapter, session support
- `services/workspace-server/src/persistence/...`
  - SeaORM repo adapters, DB bootstrap, migrations wiring
- `services/workspace-server/src/app/...`
  - wiring layer kết nối config, repos, auth và routes
- `frontends/console/src/...`
  - refine app shell, resources, auth provider, data provider và pages

## 12. Các quyết định đã chốt

- Làm một phase end-to-end, không tách thành hai phase
- Dùng `refine` tối đa cho admin shell và CRUD pattern
- Dùng auth abstraction trait-based; `Clerk` chỉ là một implementation
- Hỗ trợ `mock password auth` để dev/test/browser verify dễ dàng
- Dùng `clap` arg/env cho config server
- Dùng `SeaORM` cho persistence, SQLite cho baseline dev/test
- `dev` dùng Vite proxy `/api`
- `release` embed static console build vào `workspace-server` qua GitHub Actions
- sync export dùng machine token riêng, không dùng admin session auth
- `workspaces.last_updated` là cursor chuẩn cho incremental sync

## 13. Ngoài phạm vi hiện tại

Những mục sau không cần giải quyết sâu trong đợt này, trừ khi thật sự cần để giữ
boundary sạch:

- RBAC chi tiết hơn ngoài `SuperAdmin`, `Owner`, `Admin`, `Member`
- room domain implementation đầy đủ
- production deployment topology chi tiết
- hardening bảo mật production hoàn chỉnh cho Clerk/session/cookie
- analytics, audit log đầy đủ và notification system
