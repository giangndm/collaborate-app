# Implementation Rules

## 1. Mục tiêu

Tài liệu này là bộ rule triển khai chung cho toàn repo.

Mục tiêu của bộ rule này là giúp code có các tính chất sau:

- khó sai
- dễ review
- dễ refactor
- kiểm tra được nhiều nhất ở compile time
- tối ưu cho developer experience và performance cùng lúc

Mọi quyết định thiết kế hoặc triển khai cần được cân nhắc kỹ theo các tiêu chí
trên, thay vì làm theo thói quen hoặc tối ưu cục bộ một chiều.

---

## 2. Các nguyên tắc tối thượng

### 2.1 Compile-time over runtime

Ưu tiên mọi cách để compiler bắt lỗi sớm thay vì chờ đến runtime.

Điều này có nghĩa:

- ưu tiên typed struct, typed enum, smart constructor, trait bound rõ ràng
- ưu tiên API khó dùng sai
- tránh để invariant quan trọng chỉ được kiểm tra bằng comment hoặc convention

Rationale:

- lỗi bị chặn ở compile time là loại lỗi rẻ nhất để sửa
- code càng khó dùng sai thì dev càng làm việc nhanh và an toàn hơn

### 2.2 Make invalid states hard or impossible

State hoặc input không hợp lệ phải khó biểu diễn, hoặc tốt nhất là không thể biểu
diễn được trong type system.

Ví dụ:

- dùng `RoomId(String)` thay vì `String`
- dùng enum thay vì magic string
- dùng typed error thay vì `String`

### 2.3 Typed domain over stringly typed code

Trong domain, không dùng `String`, `&str`, `u64`, `HashMap<String, _>` như cách
biểu diễn mặc định cho identity, key, kind, state, hoặc contract nếu có thể tạo
type rõ nghĩa hơn.

Rationale:

- stringly typed code dễ viết nhanh nhưng rất dễ dùng nhầm
- typed domain giúp code tự giải thích ý nghĩa và giảm lỗi wiring

### 2.4 Refactor before duplicate

Trước khi thêm code mới, phải nghiên cứu kỹ cái đang có.

Nếu đã có pattern gần giống, ưu tiên:

- tái sử dụng
- tổng quát hóa có kiểm soát
- hoặc refactor để dùng chung

Không được tạo song song nhiều abstraction làm cùng một việc chỉ vì nhanh tay hơn.

### 2.5 Every abstraction must justify itself

Mọi abstraction mới phải trả lời được:

- nó làm code an toàn hơn ở đâu?
- nó làm code dễ hiểu hơn ở đâu?
- nó giảm duplication thật hay chỉ chuyển duplication sang nơi khác?
- chi phí của nó là gì?

Nếu không trả lời được rõ ràng, không thêm abstraction đó.

### 2.6 Làm chạy được theo cách đơn giản nhất, rồi tiếp tục đơn giản hóa

Quy trình mặc định:

- bước 1: làm cho chạy được, nhưng bằng cách đơn giản nhất có thể
- bước 2: review lại xem code đã đủ đơn giản chưa, có thể đơn giản hơn nữa không
- lặp lại cho đến khi không thể làm code dễ hơn đáng kể

Rule:

- code tốt là code được design tốt và đơn giản
- bất kỳ logic phức tạp nào cũng phải cố gắng tách thành các phần nhỏ, rõ nghĩa,
  dễ hiểu

---

## 3. Rule chung cho toàn repo

### 3.1 Phải đọc kỹ code hiện có trước khi thêm code mới

Trước khi viết code:

- tìm các type, trait, helper, pattern, adapter, module đã có
- hiểu vì sao code hiện tại được tổ chức như vậy
- chỉ thêm mới sau khi chắc chắn không thể tận dụng hoặc refactor cái cũ tốt hơn

### 3.2 Không thêm code lặp nếu có thể refactor

Nếu cùng một pattern lặp lại từ 2 lần trở lên, phải chủ động đánh giá:

- đây có phải duplication thực sự không?
- có thể gom về helper/type/trait chung không?
- nếu generic hóa, intent domain có còn rõ không?

Chỉ được giữ duplication nếu:

- hai trường hợp trông giống nhau nhưng khác nhau về intent domain
- việc generic hóa làm code khó hiểu hoặc khó kiểm tra hơn

### 3.3 Mọi quyết định phải có lý do rõ ràng

Khi chọn cách tổ chức code, luôn phải tự trả lời:

- tại sao làm vậy?
- ưu điểm là gì?
- nhược điểm là gì?
- tại sao đây là phương án tối ưu nhất cho bài toán hiện tại?

Không chấp nhận kiểu quyết định:

- “vì thường vẫn làm vậy”
- “vì framework gợi ý vậy”
- “vì nhìn có vẻ đẹp”

---

## 4. Rule cho Rust typed model

### 4.1 Domain identity phải ưu tiên newtype

Các kiểu dữ liệu như id, key, token, email, ext_id, room name, workspace name,
channel key... phải ưu tiên biểu diễn bằng typed struct/newtype.

Ví dụ tốt:

```rust
pub struct RoomId(String);
pub struct WorkspaceId(String);
pub struct MemberId(String);
```

Ví dụ không tốt:

```rust
pub type RoomId = String;
fn join_room(room_id: String)
```

Rationale:

- tránh truyền nhầm nhiều loại string có hình dạng giống nhau
- compiler giúp bắt sai wiring giữa các domain identity
- mở đường cho smart constructor và validation tập trung

### 4.2 Ưu tiên smart constructor cho invariant quan trọng

Nếu một type có invariant rõ ràng, ưu tiên tạo bằng constructor có kiểm tra thay vì
để struct public hoàn toàn không kiểm soát.

Ví dụ:

- email hợp lệ
- token không rỗng
- room id đúng format
- workspace slug đúng policy

### 4.3 Dùng derive để giảm boilerplate nhưng không làm mờ domain

Ưu tiên dùng `derive_more` và các derive phù hợp để giảm code lặp cho newtype và
typed struct.

Ví dụ phù hợp:

- `From`
- `Into`
- `AsRef`
- `Display`
- `Deref` khi thật sự hợp lý

Rule:

- derive để tăng ergonomics
- không derive bừa bãi nếu làm type mất boundary hoặc dễ bị lạm dụng như primitive

### 4.4 Enum và child enum nên tận dụng thư viện nếu thực sự giảm lặp

Có thể dùng các thư viện như `subenum`, `restructed` hoặc tương đương để:

- tạo child enum
- tái sử dụng cấu trúc dữ liệu
- giảm duplication giữa các biến thể gần nhau

Chỉ dùng khi:

- model trở nên rõ hơn
- giảm code lặp thực sự
- compiler check tốt hơn

Không dùng nếu:

- làm type flow khó hiểu
- tạo macro magic quá mức khiến code khó đọc hoặc khó debug

### 4.5 Không để domain model rò kiểu hạ tầng

Domain model không nên phụ thuộc trực tiếp vào kiểu dữ liệu của:

- database layer
- HTTP layer
- Redis adapter
- framework-specific types

Domain type phải được đặt tên theo ý nghĩa nghiệp vụ, không theo storage shape.

---

## 5. Rule cho hexagonal architecture

### 5.1 Domain không phụ thuộc adapter

Domain và application logic không được biết trực tiếp về:

- SQL
- Redis
- HTTP
- WebSocket
- Clerk
- SeaORM
- framework runtime

Những thứ đó phải đi qua port và adapter phù hợp.

### 5.2 Port phải diễn tả intent, không diễn tả công nghệ

Tên và contract của port phải phản ánh nhu cầu nghiệp vụ.

Ví dụ tốt:

- `RoomEventStore`
- `WorkspaceRepository`
- `WorkspaceSyncSource`

Ví dụ không tốt:

- `RedisThing`
- `SqlRepoGeneric`
- `DbService`

Rationale:

- intent-level port ổn định hơn implementation detail
- giúp thay adapter dễ hơn mà không kéo theo đổi nghĩa domain

### 5.3 Adapter không được nuốt logic domain

Adapter chỉ nên chịu trách nhiệm:

- chuyển đổi dữ liệu
- giao tiếp với hệ thống ngoài
- mapping lỗi hạ tầng

Adapter không được trở thành nơi chứa business rules quan trọng.

### 5.4 Không làm hexagon hình thức

Không tạo thêm trait, interface, layer, hoặc folder chỉ để “đúng mô hình hexagon”.

Nếu abstraction mới không giúp:

- tách dependency thực sự
- test tốt hơn
- compile-time safety tốt hơn
- hoặc code dễ hiểu hơn

thì không cần thêm.

---

## 6. Rule về anti-duplication và refactor

### 6.1 Ưu tiên refactor hơn là copy-paste thích nghi

Khi cần thêm behavior mới gần giống behavior cũ:

- đánh giá xem có thể refactor thành shared core không
- xem phần nào là invariant chung, phần nào là variation point
- chỉ copy khi variation lớn tới mức shared abstraction làm hại code hơn

### 6.2 Generic chỉ khi làm code tốt hơn

Generic hóa là công cụ mạnh nhưng dễ bị lạm dụng.

Chỉ generic khi nó giúp:

- giảm duplication thật
- tăng type safety
- giữ intent domain rõ
- giảm khả năng dùng sai

Không generic nếu nó gây ra:

- type signature khó đọc
- trait bound rối
- error message khó hiểu
- che mất ý nghĩa domain thật

### 6.3 Refactor là nghĩa vụ kỹ thuật, không phải việc tùy hứng

Nếu code hiện tại có thể được chỉnh lại để:

- giảm lặp
- tăng type safety
- giảm coupling
- làm cho compile-time check mạnh hơn

thì cần cân nhắc refactor trước khi thêm feature mới chồng lên kiến trúc yếu.

---

## 7. Rule về error handling và contract

### 7.1 Không dùng `String` error trong domain nếu tránh được

Domain và application layer phải ưu tiên typed error.

Ví dụ tốt:

```rust
pub enum JoinRoomError {
    RoomNotFound,
    PermissionDenied,
    InvalidToken,
}
```

Ví dụ không tốt:

```rust
Result<(), String>
```

Rationale:

- typed error giúp compiler kiểm tra flow tốt hơn
- API rõ nghĩa hơn
- caller xử lý logic dễ hơn

### 7.2 Contract phải đủ mạnh để khó dùng sai

Không dùng các contract mơ hồ nếu có thể thay bằng model mạnh hơn.

Ví dụ cần tránh:

- bool không rõ nghĩa
- `Option` khi thực chất là một state machine nhiều trạng thái
- magic string để biểu diễn action type

### 7.3 Input/output cần phản ánh intent domain

Function signature phải trả lời rõ:

- nhận cái gì?
- trả về cái gì?
- failure mode là gì?

Nếu signature không giúp người đọc hiểu được intent, cần thiết kế lại.

---

## 8. Rule về performance

### 8.1 Ưu tiên cấu trúc dữ liệu đơn giản và predictable

Chọn cấu trúc dữ liệu dựa trên đặc tính bài toán thực tế, không theo cảm tính.

Ưu tiên:

- layout đơn giản
- số allocation ít
- ownership rõ
- ít clone không cần thiết

### 8.2 Không hy sinh type safety chỉ vì tối ưu sớm

Nếu chưa có evidence rằng type safety đang là bottleneck, không được bỏ typed model
để quay về primitive yếu hơn chỉ vì “có vẻ nhẹ hơn”.

### 8.3 Hot path phải được cân nhắc kỹ hơn

Các vùng hot path như:

- room mutation
- room event apply
- replay / restore
- realtime subscription fanout

phải đặc biệt chú ý đến:

- allocation
- clone
- lock contention
- async boundary
- polling pattern

Rationale:

- kiến trúc đúng chưa đủ, cần giữ runtime cost hợp lý
- nhưng tối ưu phải đi cùng với correctness và maintainability

---

## 9. Checklist bắt buộc trước khi thêm abstraction hoặc merge code

Trước khi chốt một thiết kế hoặc code change, phải tự kiểm tra:

- Có đang dùng `String` hoặc primitive nơi đáng lẽ nên là newtype không?
- Type hiện tại đã đủ mạnh để compiler bắt lỗi chưa?
- Có đoạn code nào đang lặp mà nên refactor không?
- Abstraction mới có làm intent domain rõ hơn không?
- Abstraction mới có làm compile-time check mạnh hơn không?
- Generic hóa ở đây có thật sự giảm duplication không?
- Có đang vô tình kéo logic hạ tầng vào domain không?
- Typed error đã đủ rõ chưa?
- Trade-off hiệu năng đã được nghĩ tới chưa?

Nếu chưa trả lời được các câu hỏi trên, chưa nên merge hoặc chưa nên mở rộng thiết
kế.

---

## 10. Anti-pattern cần tránh mạnh

Các anti-pattern sau cần tránh tối đa:

- stringly typed domain ids
- `Result<T, String>` trong domain/application khi có thể dùng typed error
- duplicated model chỉ khác tên nhưng cùng intent
- generic quá sớm
- adapter chứa business rules
- trait rỗng chỉ để có “cảm giác hexagon”
- macro hoặc derive magic làm model khó hiểu hơn thay vì rõ hơn
- thêm code mới mà không đọc kỹ code cũ

---

## 11. Kết luận vận hành

Rule quan trọng nhất của repo này là:

> luôn ưu tiên thiết kế và triển khai theo hướng làm code khó sai hơn, được kiểm
> tra nhiều hơn ở compile time, ít lặp hơn, rõ domain hơn, và vẫn đủ thực dụng để
> developer làm việc nhanh và hệ thống chạy hiệu quả.

Nếu phải chọn giữa:

- nhanh nhưng yếu type
- hoặc chậm hơn một chút nhưng rõ domain và an toàn hơn

thì mặc định ưu tiên phương án an toàn và rõ ràng hơn, trừ khi có lý do hiệu năng
thực sự mạnh và đã được cân nhắc kỹ.
