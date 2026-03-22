# Implement Workspace Domain

## 1. Mục tiêu

Tài liệu này mô tả kiến trúc hexagonal chi tiết cho `workspace domain` trong
`crates/core-domain`.

Mục tiêu là chốt:

- file structure
- trách nhiệm của từng file
- chỗ nào là trait, chỗ nào là implement
- phần nào là cứng, phần nào là mềm
- phần nào mở rộng sau

Thiết kế này phải tuân theo `docs/implement_rule.md`.

---

## 2. Vai trò của workspace domain

`workspace domain` là control-plane core domain.

Nó chịu trách nhiệm cho:

- danh tính workspace
- trạng thái workspace
- chính sách nền của workspace
- metadata của credential và secret
- user control-plane tối thiểu
- mapping user <-> workspace
- vai trò của user trong từng workspace
- dữ liệu sync cần export sang gateway

Nó không chịu trách nhiệm trực tiếp cho:

- HTTP
- Clerk
- SQL/SeaORM
- Redis
- WebSocket
- framework runtime

Các thứ trên là concern của adapter.

---

## 3. Nguyên tắc thiết kế cho workspace domain

### 3.1 Tách mạnh với room domain

`workspace` và `room` là hai domain tách mạnh.

`room domain` không được import logic nội bộ của `workspace domain`.

Nếu room cần biết capability nào đó của workspace, việc đó phải đi qua typed port
hoặc typed sync context tối thiểu.

### 3.2 Permission theo scope doc/ghi, khong theo chuc danh

Domain service khong nhan kieu nhu `WorkspaceAdminPermission`.

Thay vao do, permission duoc giu toi gian theo scope thuc te cua workspace:

- `WorkspaceReadPermission { workspace_id }`
- `WorkspaceWritePermission { workspace_id }`

Ngoai le duy nhat la flow `create_workspace`: vi workspace chua ton tai `workspace_id`
nen service nhan mot guard nho `WorkspaceCreatorGuard` thay vi them mot permission type
action-specific.

Moi workspace-scoped use case hien tai, nhu doc workspace, liet ke member,
cap nhat workspace, doi membership, xoay credential metadata, hoac export sync,
deu phai dung mot trong hai permission nay thay vi tach thanh permission theo
action rieng.

Ly do:

- service signature van ro nhung it type hon
- compiler giup buoc caller di qua dung luong phan quyen
- tranh suy luan ngam kieu "admin chac lam duoc moi thu" trong service
- tranh bung no so luong permission type khi use case tang dan
- `create_workspace` van di qua guard-oriented surface ma khong can he permission rieng

### 3.3 Guard quyet dinh quyen, service chi nhan permission

Logic ai duoc lam gi khong nen nam truc tiep trong service.

Flow dung:

```text
verified actor context
  -> Guard
  -> `WorkspaceCreatorGuard` hoac `WorkspaceReadPermission` / `WorkspaceWritePermission`
  -> workspace service method
```

Guard thuong duoc tao tu tang ngoai sau khi verify token, session, secret hoac
identity context.

### 3.4 Workspace role là mapping ngoài

Một user có thể thuộc nhiều workspace với vai trò khác nhau, nên `workspace role`
không được nhét trực tiếp vào `User`.

Role theo workspace phải nằm ở `WorkspaceMembership`.

Baseline role:

- `Owner`
- `Member`

---

## 4. Cấu trúc thư mục đề xuất

```text
crates/core-domain/src/domain/workspace/
  mod.rs

  types/
    mod.rs
    ids.rs
    user.rs
    membership.rs
    permissions.rs
    status.rs
    policy.rs
    credentials.rs
    sync.rs
    errors.rs

  entity/
    mod.rs
    workspace.rs
    user.rs
    membership.rs

  guards/
    mod.rs
    workspace_member_guard.rs
    super_admin_guard.rs

  ports/
    mod.rs
    workspace_repository.rs
    user_repository.rs
    membership_repository.rs
    secret_store.rs

  service/
    mod.rs
    workspace_service.rs
    sync_service.rs
```

Nguyên tắc:

- `types/` chứa type mạnh và contract domain
- `entity/` chứa behavior và invariant cốt lõi
- `guards/` chứa role/context -> permission logic
- `ports/` chỉ chứa trait dependency ra ngoài
- `service/` chứa orchestration use case

---

## 5. Chi tiết từng nhóm file

## 5.1 `mod.rs`

### Mục đích

- re-export có chọn lọc public API của workspace domain
- không chứa logic nghiệp vụ

### Cứng hay mềm

- mềm

### Ghi chú

- public surface phải gọn, không export tràn lan toàn bộ internal module

---

## 5.2 `types/`

`types/` là phần rất cứng của domain vì nó định nghĩa boundary ở compile time.

### `types/ids.rs`

Chứa các identity newtype, ví dụ:

- `WorkspaceId`
- `UserId`
- `WorkspaceMembershipId`
- `WorkspaceApiKeyId`
- `WorkspaceSecretVersion`

Rule:

- ưu tiên `derive_more`
- tránh alias primitive kiểu `type WorkspaceId = String`
- nếu có invariant format, dùng smart constructor

### `types/user.rs`

Chứa user-level control-plane types, ví dụ:

- `GlobalUserRole`
- `UserStatus`
- `UserEmail`
- `DisplayName`

Baseline role toàn cục:

- `Member`
- `SuperAdmin`

### `types/membership.rs`

Chứa role ở mức workspace membership.

Baseline:

- `WorkspaceRole::Owner`
- `WorkspaceRole::Member`

Đây là file rất cứng vì ảnh hưởng trực tiếp đến permission flow.

### `types/permissions.rs`

Chứa permission types tối giản cho workspace domain.

Ví dụ:

- `WorkspaceReadPermission { workspace_id }`
- `WorkspaceWritePermission { workspace_id }`

Rule:

- `WorkspaceCreatorGuard` la case rieng vi luc tao workspace chua co `workspace_id`
- mọi workspace-scoped action còn lại chỉ dùng read hoặc write permission
- không tạo permission type riêng cho invite, credential, sync export, hoặc các
  action tương tự nếu chúng vẫn chỉ là biến thể của read/write access

Đây là file rất quan trọng vì service API sẽ nhận trực tiếp các type này.

### `types/status.rs`

Chứa trạng thái domain cốt lõi, ví dụ:

- `WorkspaceStatus`
- có thể thêm trạng thái khác nếu thấy cần tách khỏi `user.rs`

Ví dụ baseline cho workspace:

- `Active`
- `Suspended`
- `Disabled`

### `types/policy.rs`

Chứa workspace-level policy ảnh hưởng tới gateway và room access, ví dụ:

- guest allowed hay không
- default token ttl
- required claims

Phần này cứng ở shape cơ bản, nhưng mềm hơn ở chi tiết field vì sẽ mở rộng theo
use case.

### `types/credentials.rs`

Chứa metadata về credential và secret profile.

Lưu ý:

- domain không nên giữ raw secret như một field bình thường trong aggregate nếu có
  thể tránh
- nên biểu diễn bằng metadata hoặc reference

Ví dụ:

- `WorkspaceApiKeyMetadata`
- `WorkspaceSigningProfile`
- `WorkspaceSecretRef`

### `types/sync.rs`

Chứa `WorkspaceSyncPayload` hoặc type tương đương.

Đây là contract rất cứng vì nó là seam giữa `Console` và `Gateway`.

Nó chỉ nên chứa dữ liệu thật sự cần cho gateway hot path.

### `types/errors.rs`

Chứa typed error dùng `thiserror`.

Rule:

- không dùng `String` error ở domain/service nếu tránh được
- error nên mang context typed để dễ trace

Ví dụ:

```rust
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace not found: {workspace_id}")]
    WorkspaceNotFound { workspace_id: WorkspaceId },

    #[error("permission denied for user {user_id} on workspace {workspace_id}")]
    PermissionDenied {
        user_id: UserId,
        workspace_id: WorkspaceId,
        action: &'static str,
    },
}
```

---

## 5.3 `entity/`

`entity/` chứa core behavior và invariant của domain.

### `entity/workspace.rs`

Chứa aggregate `Workspace`.

Nó nên giữ:

- id
- status
- policy
- credential metadata
- version hoặc timestamp domain-level nếu cần

Ví dụ behavior:

- `activate`
- `suspend`
- `disable`
- `update_policy`
- `rotate_credential_metadata`

Đây là file rất cứng.

### `entity/user.rs`

Chứa `User` entity ở mức control-plane core.

Nó nên giữ:

- `UserId`
- global role
- status
- profile tối thiểu nếu thật sự là domain concern

Không nên nhét workspace role vào đây.

### `entity/membership.rs`

Chứa `WorkspaceMembership`.

Nó là mapping giữa:

- `UserId`
- `WorkspaceId`
- `WorkspaceRole`

Nó có thể chứa behavior như:

- đổi role
- revoke membership
- validate chuyển role hợp lệ

Đây là file cứng vì nó chốt quan hệ user <-> workspace.

---

## 5.4 `guards/`

`guards/` là nơi chuyển actor context thành typed permission.

Guard thường được tạo sau khi tầng ngoài đã verify identity.

### `guards/workspace_member_guard.rs`

Chứa guard cho actor đang là member của một workspace cụ thể.

Context tối thiểu có thể gồm:

- `user_id`
- `workspace_id`
- `workspace_role`
- `user_status` nếu cần

Guard này sẽ hỗ trợ:

- `read_permission() -> WorkspaceReadPermission`
- `write_permission() -> Option<WorkspaceWritePermission>`

### `guards/super_admin_guard.rs`

Chứa guard cho actor có global role `SuperAdmin`.

Guard này có thể:

- tạo permission cho bất kỳ workspace nào khi được cung cấp `workspace_id`
- bypass một phần membership check theo rule nghiệp vụ được chốt

Ví dụ:

```text
SuperAdminGuard + workspace_id
  -> WorkspaceWritePermission { workspace_id }
```

### Guard là cứng hay mềm?

- logic mapping role -> permission là khá cứng
- chi tiết context field có thể mềm hơn

---

## 5.5 `ports/`

`ports/` chỉ chứa trait dependency ra ngoài domain.

Không chứa implement.

### `ports/workspace_repository.rs`

Trait để load/save `Workspace`.

Ví dụ trách nhiệm:

- get by id
- save
- list updated since

### `ports/user_repository.rs`

Trait để load/save `User`.

Ví dụ trách nhiệm:

- get by id
- save
- list active users nếu cần cho use case quản trị

### `ports/membership_repository.rs`

Trait để load/save `WorkspaceMembership`.

Ví dụ trách nhiệm:

- get membership theo `workspace_id + user_id`
- list members của workspace
- save membership

### `ports/secret_store.rs`

Trait cho việc lấy secret material hoặc secret reference khi nghiệp vụ cần.

Mục tiêu của port này là giữ boundary rõ giữa:

- domain metadata
- secret material nhạy cảm

Port này là phần mềm hơn, vì cách ta lưu secret có thể đổi theo hạ tầng.

---

## 5.6 `service/`

`service/` chứa orchestration use case của workspace domain.

### `service/workspace_service.rs`

Chứa service chính cho workspace use cases.

Các method của service phải nhận `typed permission` thay vì role thô.

Ví dụ:

- `create_workspace(guard: &WorkspaceCreatorGuard, ...)`
- `read_workspace(permission: &WorkspaceReadPermission, ...)`
- `read_member_user(permission: &WorkspaceReadPermission, ...)`
- `list_members(permission: &WorkspaceReadPermission, ...)`
- `save_workspace(permission: &WorkspaceWritePermission, ...)`
- `save_membership(permission: &WorkspaceWritePermission, ...)`

Các use case như invite member, change member role, rotate credential metadata,
hoặc export sync không cần permission type riêng nếu chỉ yêu cầu read/write
scope của cùng workspace.

Ví dụ flow đúng:

```text
verified actor context
  -> guard
  -> `WorkspaceReadPermission` hoặc `WorkspaceWritePermission`
  -> workspace service method
```

### `service/sync_service.rs`

Chứa nghiệp vụ `export sync payload` cho gateway.

Đây là service riêng vì sync export là use case quan trọng, không chỉ là chi tiết
port.

Service này có thể dùng:

- repository
- secret_store
- `WorkspaceSyncPayload`

### Service là trait hay struct?

Khuyến nghị baseline:

- `ports/` là trait
- `service/` là `struct` thật với dependency injected qua trait object hoặc generic
  phù hợp

Không nên tạo thêm trait cho service nếu chưa có nhu cầu thực tế.

Lý do:

- tránh hexagon hình thức
- service là use case orchestration, không phải dependency boundary mặc định

---

## 6. Public API đề xuất của workspace domain

`workspace/mod.rs` nen re-export co chon loc cac thanh phan sau:

- ids và typed value objects quan trọng
- `Workspace`, `User`, `WorkspaceMembership`
- `WorkspaceRole`, `GlobalUserRole`
- permission types
- guard types chính
- repository traits
- service structs
- error types
- sync payload type

Không nên export toàn bộ internal helper hoặc module detail.

---

## 7. Cái gì là cứng, cái gì là mềm

### 7.1 Cứng

- identity newtypes
- `GlobalUserRole`
- `WorkspaceRole`
- `WorkspaceCreatorGuard`
- `WorkspaceReadPermission`
- `WorkspaceWritePermission`
- `Workspace`, `User`, `WorkspaceMembership`
- `WorkspaceSyncPayload`
- nguyên tắc guard -> permission -> service

### 7.2 Mềm

- chia repository thành bao nhiêu file nhỏ hơn nữa
- số lượng use case service chi tiết
- mức chi tiết của policy và credential metadata
- shape của một số helper type không chạm invariant chính

### 7.3 Để mở rộng sau

- invite entity riêng với lifecycle đầy đủ
- audit events cho workspace
- nhiều API keys per workspace
- advanced policy inheritance
- user activation workflow sâu hơn
- richer sync filter cho gateway

---

## 8. Rule trait ở đâu, implement ở đâu

### Trait đặt ở đâu

- chỉ đặt trait dependency ở `ports/`
- trait phải mô tả intent nghiệp vụ, không mô tả công nghệ

### Implement đặt ở đâu

- behavior cốt lõi đặt ở `entity/`
- orchestration use case đặt ở `service/`
- guard conversion đặt ở `guards/`
- adapter implement thật đặt ngoài `core-domain`, ví dụ ở `services/workspace-server/`

### Không làm gì

- không đặt SQL implement trong `core-domain`
- không đặt HTTP request/response model trong `workspace domain`
- không để service phụ thuộc trực tiếp SeaORM/Clerk/framework types

---

## 9. Ví dụ flow phân quyền đúng

### 9.0 Actor tao workspace moi

```text
actor context
  -> WorkspaceCreatorGuard
  -> WorkspaceService::create_workspace(guard, ...)
```

### 9.1 Member doc workspace

```text
actor context
  -> WorkspaceMemberGuard
  -> read_permission() -> WorkspaceReadPermission { workspace_id }
  -> WorkspaceService::read_workspace(permission, ...)
```

### 9.2 Owner mời member vào workspace

```text
actor context
  -> WorkspaceMemberGuard
  -> write_permission() -> WorkspaceWritePermission { workspace_id }
  -> WorkspaceService::<workspace write use case>(permission, ...)
```

### 9.3 Super admin cập nhật workspace bất kỳ

```text
actor context
  -> SuperAdminGuard
  -> write_permission(workspace_id) -> WorkspaceWritePermission { workspace_id }
  -> WorkspaceService::save_workspace(permission, ...)
```

---

## 10. Kết luận

Thiết kế workspace domain này theo đúng tinh thần:

- hexagonal thật, không hình thức
- strong typed domain
- compile-time safety cao
- permission workspace-scoped được giữ tối giản theo read/write
- guard quyết định quyền, service chỉ nhận quyền đúng type
- user và membership được model đúng với bài toán nhiều workspace

Nó là nền đủ mạnh để triển khai tiếp các use case control-plane mà không kéo room
domain hoặc hạ tầng vào domain core.
