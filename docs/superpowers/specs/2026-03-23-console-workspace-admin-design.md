# Thiết kế Console Admin và Workspace Server

## 1. Mục tiêu

Tài liệu này chốt thiết kế end-to-end cho `workspace-server` và `console frontend`
để tạo ra một control plane hoàn chỉnh cho quản lý workspace.

Phạm vi bao gồm:

- admin console nhiều trang theo phong cách `refine`
- `workspace-server` có HTTP API và wiring đầy đủ vào `core-domain`
- repository dùng `SeaORM` theo hướng type-safe cho dữ liệu workspace
- auth abstraction với implementation ở đợt này là `mock password auth`
- bản release embed static build của `console frontend` vào `workspace-server`
- test đầy đủ và browser verification bằng Chrome MCP cho các luồng quản trị chính

Tài liệu này ưu tiên các nguyên tắc sau:

- kiến trúc hexagonal cho Rust backend
- compile-time safety và typed domain boundary
- giảm duplication giữa domain, repo, HTTP và frontend data flow
- DX tốt cho local development, test và review

## 2. Kết quả cần đạt

Sau khi hoàn thành, repo cần có các tính chất sau:

- admin đăng nhập bằng `mock password login` trong đợt này
- `Clerk` được giữ như hướng mở rộng tiếp theo, chưa triển khai trong phạm vi hiện tại
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
- `add_member`
- `change_member_role`
- `remove_member`
- `list_credentials`
- `create_credential`
- `rotate_secret`
- `export_sync_payload`

Luồng authz vẫn theo cấu trúc:

```text
admin HTTP request
  -> authenticated actor context
  -> guard
  -> WorkspaceCreatorGuard / WorkspaceReadPermission / WorkspaceWritePermission
  -> WorkspaceService

sync HTTP request
  -> machine token
  -> IntegrationGuard
  -> WorkspacesReadPermission
  -> WorkspaceSyncService (sau khi hoàn tất task sync riêng)
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
  use case `list_members`, `add_member`, `change_member_role`, và `remove_member`
  có baseline owner hợp lệ ngay từ đầu

Điều này cũng chốt luôn hành vi UI:

- nút `Create workspace` chỉ hiển thị cho actor có `global_role = SuperAdmin`
- seed test cần có ít nhất một mock account `SuperAdmin` để cover full create flow

Rule visibility cho `list_workspaces_visible_to_actor` được chốt như sau:

- workspace roles trong đợt này chỉ còn: `Owner` và `Member`

- `SuperAdmin` nhìn thấy tất cả workspace
- actor có workspace membership nhìn thấy tất cả workspace mà mình là member
- trong đợt này, mọi workspace member đều có cùng rule visibility; khác biệt role ở
  mức workspace chỉ còn `Owner` và `Member`

Permission matrix baseline cho mutate actions:

- `update_workspace`
  - `SuperAdmin`: được phép
  - `Owner`: được phép trên workspace của mình
  - `Member`: không được phép
- `create_credential`
  - `SuperAdmin`: được phép
  - `Owner`: được phép trên workspace của mình
  - `Member`: không được phép
- `rotate_secret`
  - `SuperAdmin`: được phép
  - `Owner`: được phép trên workspace của mình
  - `Member`: không được phép
- `add_member`
  - `SuperAdmin`: được phép
  - `Owner`: được phép trên workspace của mình
  - `Member`: không được phép
- `remove_member`
  - `SuperAdmin`: được phép
  - `Owner`: được phép trên workspace của mình
  - `Member`: không được phép

Đợt này cố ý giữ mutate permission đơn giản theo hướng `SuperAdmin` hoặc
`Owner`-only để giảm ambiguity trong domain và UI.

Rule bổ sung cho owner-management:

- `Owner` không được xoá một `Owner` khác trong đợt này
- chỉ `SuperAdmin` mới có thể xoá một `Owner`, và vẫn phải tuân thủ rule không được
  xoá `Owner` cuối cùng

### 4.3 Auth abstraction

Backend giới thiệu một auth boundary mới để `workspace-server` không bị khóa chặt
vào một cơ chế auth cụ thể.

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

Implementation trong đợt này chỉ có một provider:

1. `Mock password auth`
   - frontend submit username/password tới backend
   - backend tạo session cookie `HttpOnly`
   - backend đọc cookie và map sang `AuthenticatedActor`

Thiết kế vẫn giữ `AuthSessionResolver` và `AuthProviderKind` để sau này có thể thêm
`Clerk` như một `next improvement`, nhưng không có yêu cầu triển khai Clerk trong
đợt hiện tại.

### 4.4 Chuẩn hóa session API

Để frontend refine auth provider có contract ổn định, session API được chốt như
sau:

Session model baseline được chốt như sau:

- dùng server-side session store trong SQLite để đơn giản hóa implementation và
  logout invalidation
- cookie tên `workspace_console_session`
- cookie là `HttpOnly`, `SameSite=Lax`
- session TTL mặc định: 7 ngày
- `POST /api/auth/logout` phải xoá session record tương ứng và clear cookie

- `GET /api/auth/session`
  - trong đợt này: đọc cookie session và trả actor normalized
- `POST /api/auth/logout`
  - trong đợt này: xóa session cookie và trả `204`
- `POST /api/auth/password/login`
  - nhận username/password, tạo session cookie và trả actor normalized

Contract tối thiểu cho login endpoint:

- request body:

```json
{
  "username": "superadmin",
  "password": "supersecret"
}
```

- success response:

```json
{
  "data": {
    "user_id": "usr_superadmin",
    "display_name": "Demo Super Admin",
    "email": "superadmin@example.com",
    "global_role": "super_admin",
    "auth_provider": "mock_password"
  }
}
```

- error response:

```json
{
  "error": {
    "code": "invalid_credentials",
    "message": "Tên đăng nhập hoặc mật khẩu không đúng"
  }
}
```

- unauthenticated response của `GET /api/auth/session`:

```json
{
  "error": {
    "code": "unauthenticated",
    "message": "Bạn chưa đăng nhập"
  }
}
```

với HTTP status `401`.

Như vậy frontend luôn dùng cùng một `session-check flow` cho mock auth. Contract
này vẫn đủ ổn định để sau này thêm provider mới mà không phải thay toàn bộ refine
auth provider.

### 4.5 Persistence với SeaORM

`SeaORM` là adapter chính cho `workspace-server` vì cần DB-agnostic và query
surface có type rõ ràng. Bản đầu dùng SQLite để giảm complexity.

Config DB:

- `dev`: SQLite file on disk
- `test`: SQLite in-memory

Schema tối thiểu:

- `users`
- `auth_sessions`
- `workspaces`
- `workspace_memberships`
- `workspace_credentials`
- `workspace_credential_secret_versions`

Trong đó bảng `users` tối thiểu phải có đủ field để dựng session và permission
resolution:

- `user_id`
- `email`
- `display_name`
- `global_role`
- `status`

`auth_sessions` tối thiểu cần có các field sau:

- `session_id`
- `user_id`
- `expires_at`
- `created_at`
- `revoked_at`

Nguyên tắc adapter:

- SeaORM entity/model chỉ sống trong adapter layer
- repository adapter map sang typed domain entities/value objects
- migrations được chạy từ `workspace-server` startup hoặc test bootstrap theo
  config

Mock auth accounts không được lưu trong database.

Thay vào đó, chúng là static list truyền từ `clap` arg hoặc env khi khởi động
`workspace-server`.

Mỗi phần tử mock auth account tối thiểu gồm:

- `username`
- `password`
- `user_id`

Config shape được chốt để tránh mơ hồ trong wiring:

- `--mock-auth-account` có thể truyền nhiều lần qua `clap`
- mỗi giá trị theo format: `<username>:<password>:<user_id>`
- env tương đương: `WORKSPACE_SERVER_MOCK_AUTH_ACCOUNTS`
- env dùng danh sách phân tách bởi dấu phẩy, ví dụ:
  `superadmin:supersecret:usr_superadmin,ownerdemo:demo123:usr_owner_demo`

Rule runtime:

- mock login chỉ so khớp với static list này
- mỗi mock account trỏ tới một `user_id` đã tồn tại trong seed dữ liệu app
- seed `users` và `workspace_memberships` vẫn nằm trong SQLite; chỉ riêng thông tin
  password login được cấp từ config runtime để đơn giản hóa hệ thống

Nếu sau này thêm `Clerk`, có thể bổ sung bảng ánh xạ external identity sang local
`user_id` mà không phá vỡ boundary auth hiện tại. Tuy nhiên phần đó nằm ngoài phạm
vi của đợt này.

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
- users
  - `GET /api/workspaces/:workspace_id/member-candidates?query=<text>`
- workspaces
  - `GET /api/workspaces`
  - `POST /api/workspaces`
  - `GET /api/workspaces/:workspace_id`
  - `PATCH /api/workspaces/:workspace_id`
- members
  - `GET /api/workspaces/:workspace_id/members`
  - `POST /api/workspaces/:workspace_id/members`
  - `PATCH /api/workspaces/:workspace_id/members/:user_id`
  - `DELETE /api/workspaces/:workspace_id/members/:user_id`
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

- rule này áp dụng cho admin CRUD endpoints; sync endpoints là ngoại lệ và dùng
  cursor contract riêng ở phần `5.3`

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

`GET /api/workspaces` item DTO:

```json
{
  "workspace_id": "ws_123",
  "name": "Acme Workspace",
  "slug": "acme-workspace",
  "status": "active",
  "last_updated": "2026-03-23T10:00:00Z"
}
```

`GET /api/workspaces/:workspace_id` response:

```json
{
  "data": {
    "workspace_id": "ws_123",
    "name": "Acme Workspace",
    "slug": "acme-workspace",
    "status": "active",
    "last_updated": "2026-03-23T10:00:00Z",
    "default_room_policy": {
      "guest_join_enabled": false,
      "token_ttl_seconds": 3600
    }
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

`GET /api/workspaces/:workspace_id/member-candidates?query=<text>` response item DTO:

```json
{
  "user_id": "usr_456",
  "email": "member@example.com",
  "display_name": "Workspace Member",
  "global_role": "member",
  "user_status": "active"
}
```

Endpoint này dùng để tìm user khi thêm member từ giao diện admin. Trong đợt này,
frontend `Members / Add` dùng search box đơn giản gọi endpoint này rồi chọn một user
để thêm vào workspace.

Authorization rule cho endpoint này:

- `SuperAdmin` được gọi endpoint này trên mọi workspace
- `Owner` được gọi endpoint này trên workspace của mình
- `Member` không được gọi endpoint này trong đợt này

Candidate set được chốt như sau:

- chỉ trả về `users` có `status = active`
- loại trừ mọi user đã là member của `workspace_id` hiện tại
- nếu caller là `SuperAdmin`, backend search theo `email` hoặc `display_name` với
  phép match `contains`, không phân biệt hoa thường
- nếu caller là `Owner`, backend chỉ cho phép search theo `email` với phép match
  `prefix`, không phân biệt hoa thường, và query phải có tối thiểu 3 ký tự
- nếu `Owner` gửi query ngắn hơn 3 ký tự, backend trả `400` với mã lỗi
  `query_too_short`
- nếu cố `add_member` với một `user_id` đã là member sẵn, backend trả lỗi domain
  `member_already_exists`

Response envelope của endpoint này dùng cùng contract list chuẩn:

```json
{
  "data": [
    {
      "user_id": "usr_456",
      "email": "member@example.com",
      "display_name": "Workspace Member",
      "global_role": "member",
      "user_status": "active"
    }
  ],
  "total": 1,
  "page": 1,
  "per_page": 20
}
```

`POST /api/workspaces/:workspace_id/members` request:

```json
{
  "user_id": "usr_456",
  "workspace_role": "member"
}
```

`POST /api/workspaces/:workspace_id/members` response:

```json
{
  "data": {
    "user_id": "usr_456",
    "email": "member@example.com",
    "display_name": "Workspace Member",
    "workspace_role": "member",
    "user_status": "active"
  }
}
```

Policy của `change_member_role` được chốt như sau:

- `SuperAdmin` có thể đổi role của mọi member trong mọi workspace
- `Owner` có thể đổi role của `Member` trong workspace của mình
- `Member` không có quyền đổi role
- không được phép demote hoặc xoá `Owner` cuối cùng của một workspace
- chỉ `SuperAdmin` mới có thể gán hoặc thăng cấp `workspace_role = owner`
- `Owner` không được tự tạo thêm `Owner` mới trong đợt này

Rule này là baseline an toàn để giữ invariant ownership đơn giản trong đợt hiện tại.

`DELETE /api/workspaces/:workspace_id/members/:user_id` response:

- trả `204` khi xoá thành công
- nếu cố xoá `Owner` cuối cùng thì trả lỗi domain tương ứng

`PATCH /api/workspaces/:workspace_id/members/:user_id` request:

```json
{
  "workspace_role": "member"
}
```

`PATCH /api/workspaces/:workspace_id/members/:user_id` response:

```json
{
  "data": {
    "user_id": "usr_123",
    "email": "owner@example.com",
    "display_name": "Workspace Owner",
    "workspace_role": "member",
    "user_status": "active"
  }
}
```

`POST /api/workspaces/:workspace_id/credentials` request:

```json
{
  "label": "Gateway integration key"
}
```

`credential` list/detail DTO:

```json
{
  "api_key_id": "key_123",
  "label": "Gateway integration key",
  "status": "active",
  "version": 2,
  "created_at": "2026-03-23T10:00:00Z",
  "rotated_at": "2026-03-24T10:00:00Z"
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
- baseline hiện tại: token toàn cục được quyền đọc tất cả workspace
- request sai token trả `401`
- workspace không tồn tại trả `404`
- hardening như per-workspace token hoặc allowlist để sau, không nằm trong đợt này

Trong đợt này, `syncable workspace` được định nghĩa là mọi workspace. Điều này đảm
bảo gateway vẫn nhận được thay đổi khi một workspace chuyển từ `Active` sang trạng
thái không hoạt động.

### 5.2 Sync guard boundary

Để không làm mờ typed permission direction của domain:

- global sync token được xử lý ở app layer rồi map sang một domain guard riêng cho
  trusted internal communication, `IntegrationGuard`
- `IntegrationGuard` không thay thế auth model của admin session
- sync flow không dùng `WorkspaceReadPermission` vì endpoint sync cần đọc nhiều
  workspace và không gắn trước với một `workspace_id` cụ thể
- domain/app layer bổ sung `WorkspacesReadPermission` để diễn tả quyền đọc tất cả
  workspace syncable trong bối cảnh machine-to-machine pull
- `IntegrationGuard` tạo `WorkspacesReadPermission` sau khi verify machine token
- trong quá trình triển khai có thể giới thiệu `IntegrationGuard` và
  `WorkspacesReadPermission` trước, sau đó mới chuyển `WorkspaceSyncService` sang
  surface này ở task sync riêng để giữ từng bước refactor nhỏ và an toàn
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

Response shape của `GET /api/sync/workspaces` được chốt như sau:

```json
{
  "data": [
    {
      "workspace_id": "ws_123",
      "last_updated": "2026-03-23T10:00:00Z"
    }
  ],
  "limit": 100,
  "has_more": false,
  "next_cursor": {
    "updated_after": "2026-03-23T10:00:00Z",
    "after_workspace_id": "ws_123"
  }
}
```

Với empty page, `data` là mảng rỗng, `has_more = false`, và `next_cursor` giữ nguyên
cursor đầu vào hoặc là `null` nếu request đầu tiên không truyền cursor.

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
- auth provider cho `password login`
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
- `Members / Add + Update role + Remove`
- `Credentials / List + Create + Rotate secret`

Phong cách UI:

- nhiều trang, dễ đọc, rõ ràng, ưu tiên admin productivity
- không theo hướng marketing landing page
- gần với refine default admin shell hơn là một theme custom phức tạp

### 6.4 Data và auth trên frontend

Frontend sử dụng:

- refine `data provider` nối vào `/api/...`
- refine `auth provider` cho mock password session

Đợt này không cần runtime config cho auth mode vì chỉ có một auth method thực thi
là password login.

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
- sync token settings
- static asset mode
- migration/seed mode

Seed baseline cho `dev/test`:

- 2 mock account
  - `superadmin:supersecret:usr_superadmin`
  - `member-owner-demo:demo123:usr_owner_demo`
- 1-2 workspace demo
- membership đủ để cover list/detail/member/credential flow

Nếu `mock auth` được bật, config sẽ nhận static list account qua arg/env để dễ
khởi động local và test mà không cần thêm persistence riêng cho password.

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
   - auth screen behavior của password login
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
  - auth abstraction, mock adapter, session support
- `services/workspace-server/src/persistence/...`
  - SeaORM repo adapters, DB bootstrap, migrations wiring
- `services/workspace-server/src/app/...`
  - wiring layer kết nối config, repos, auth và routes
- `frontends/console/src/...`
  - refine app shell, resources, auth provider, data provider và pages

## 12. Các quyết định đã chốt

- Làm một phase end-to-end, không tách thành hai phase
- Dùng `refine` tối đa cho admin shell và CRUD pattern
- Dùng auth abstraction trait-based; đợt này chỉ implement `mock password auth`
- Hỗ trợ `mock password auth` để dev/test/browser verify dễ dàng
- Giữ `Clerk` như next improvement, không triển khai trong đợt này
- Dùng `clap` arg/env cho config server
- Dùng `SeaORM` cho persistence, SQLite cho baseline dev/test
- `dev` dùng Vite proxy `/api`
- `release` embed static console build vào `workspace-server` qua GitHub Actions
- sync export dùng machine token riêng, không dùng admin session auth
- `workspaces.last_updated` là cursor chuẩn cho incremental sync

## 13. Ngoài phạm vi hiện tại

Những mục sau không cần giải quyết sâu trong đợt này, trừ khi thật sự cần để giữ
boundary sạch:

- RBAC chi tiết hơn ngoài `SuperAdmin`, `Owner`, `Member`
- room domain implementation đầy đủ
- production deployment topology chi tiết
- triển khai `Clerk` adapter và UI login tương ứng
- hardening bảo mật production hoàn chỉnh cho Clerk/session/cookie
- analytics, audit log đầy đủ và notification system
