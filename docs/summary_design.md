# Tài liệu tổng hợp thiết kế

## Nền tảng room realtime đa tenant với state có khả năng sync, record và restore

---

## 1. Mục đích và vai trò của tài liệu

Tài liệu này là `summary design` ở mức `technical pre-spec` cho nền tảng room
realtime đa tenant.

Mục đích của tài liệu:

- chốt các quyết định kiến trúc cấp cao
- thống nhất mô hình tư duy cho các tài liệu tiếp theo
- làm đầu vào cho các bước sau: `system design`, `skeleton`, và `coding`

Tài liệu này không có mục tiêu:

- thay thế detailed system design
- chốt module breakdown chi tiết
- chốt protocol, schema, storage layout, hay deployment topology cuối cùng

Nó tập trung vào các `architectural commitments`, boundary chính, mô hình xử lý,
và những ràng buộc mà các bước sau phải tôn trọng.

---

## 2. Phạm vi sản phẩm và mục tiêu thiết kế

Hệ thống hướng tới một nền tảng cho các ứng dụng tương tác theo room, trong đó
mỗi room là một thực thể realtime có khả năng xử lý state và đồng bộ cho nhiều
người tham gia.

Hệ thống cần hỗ trợ nhiều loại bài toán như:

- collaboration room
- game room
- meeting room
- AI-assisted interaction room

### 2.1 Mục tiêu chức năng

- Hỗ trợ `workspace` đa tenant với identity, token và quyền riêng
- Hỗ trợ `room` là đơn vị thực thi realtime chính
- Hỗ trợ client gửi `mutation` để tác động lên room
- Hỗ trợ client `subscribe` theo từng phần state liên quan
- Hỗ trợ `record`, `replay`, và `restore` room state

### 2.2 Mục tiêu phi chức năng

- Độ trễ thấp cho update realtime
- Deterministic room logic
- Khả năng scale theo số room và số node
- Khả năng phục hồi sau restart, crash, hoặc move room sang node khác
- Khả năng mở rộng để cắm room logic mới vào cùng một engine chung

### 2.3 Out of scope của tài liệu này

- Chưa chốt schema chi tiết cho từng room type
- Chưa chốt topology multi-node cuối cùng
- Chưa chốt network protocol cụ thể
- Chưa chốt backend CRDT duy nhất cho toàn hệ thống

---

## 3. Các cam kết kiến trúc

Đây là các quyết định nền tảng mà những tài liệu và implementation sau này phải
tuân theo.

### 3.1 Room là execution boundary chính

`Room` là đơn vị thực thi của data plane. Mỗi room có:

- state riêng
- tập participants riêng
- lifecycle riêng
- history riêng
- subscription graph riêng

Hệ thống có thể có nhiều room, nhưng hành vi realtime được định nghĩa và xử lý
theo từng room.

### 3.2 Thiết kế true multi-writer ngay từ đầu

Hệ thống được thiết kế theo giả định có thể có nhiều actor hoặc nhiều node tạo
ra thay đổi đồng thời trên cùng một room.

Điều này có nghĩa:

- không được dựa vào giả định single-writer cứng cho mỗi room như một chân lý
  kiến trúc
- state model phải có khả năng merge và hội tụ
- replay, restore, và replication phải song hành với nhau trong cùng mô hình

### 3.3 State engine phải CRDT-compatible

Canonical room state phải được đặt trên một state engine có các tính chất phù
hợp với collaboration realtime:

- sync được
- record được
- replay được
- restore được
- hội tụ được dưới concurrent writes hợp lệ

Tài liệu này không ràng buộc kiến trúc vào một thư viện duy nhất, nhưng state
engine phải đáp ứng các tính chất trên.

### 3.4 Automerge là reference implementation

Trong giai đoạn hiện tại, `Automerge` được xem là `reference implementation` để
xác thực hướng kiến trúc:

- state có thể sync
- thay đổi state có thể được record
- room có thể replay và restore
- concurrent changes có thể merge

Tuy nhiên, kiến trúc không bị khóa cứng vào API hoặc internal model riêng của
Automerge.

### 3.5 Room event là state-change record sinh tự động

Trong kiến trúc này, `room event` không được định nghĩa chủ yếu như business
event thủ công kiểu `UserJoined` hay `MessageSent`.

Thay vào đó:

- `room event` là bản ghi thay đổi state của room
- `room event` được sinh tự động trong quá trình xử lý mutation
- dev room không phải tự tay tạo event cho tất cả luồng realtime

Điều này giúp developer tập trung vào room logic theo mô hình:

`validate -> mutate state -> platform tự record + sync + publish`

### 3.6 Audit log là lớp riêng

Hệ thống có hai nhóm bản ghi khác nhau:

- `room event`: execution history chính của room realtime
- `audit log`: bản ghi cho audit, tracing, compliance, operational review

`audit log` là một concern riêng và không thay thế room event history.

---

## 4. Mô hình miền cơ bản

### 4.1 Workspace

`Workspace` là boundary của tenant.

Mỗi workspace quản lý:

- admin
- token
- policy
- room catalog
- quota hoặc ownership rules trong phạm vi tenant

Workspace thuộc control plane.

### 4.2 Room

`Room` là đơn vị xử lý realtime trong data plane.

Mỗi room có:

- `room_id`
- room state hiện tại
- tập participant/member
- tập subscription hiện tại
- room event history
- snapshot gần nhất hoặc phù hợp

Có thể hình dung room như sau:

```text
Room
 |- Identity
 |- Membership
 |- State
 |- Channels
 |- Subscriptions
 |- Room Event Log
 `- Snapshots
```

### 4.3 Room state

`Room state` là canonical materialized state của room tại một thời điểm.

Trong thiết kế này, room state không được xem như một khối lớn duy nhất. Bản chất
của nó là tập hợp của nhiều `state con`, mỗi state con đại diện cho một concern
hoặc một miền dữ liệu riêng trong room.

Đây là state mà room logic đọc và mutate trong quá trình xử lý mutation.

Yêu cầu bắt buộc:

- state phải do platform quản lý
- state phải sync được
- state phải record được
- state phải replay được
- state phải restore được

State nào không đáp ứng được các tính chất trên không được trở thành một phần
của canonical room behavior.

Có thể hình dung như sau:

```text
Room State
 |- participant state
 |- chat state
 |- presence state
 |- game state
 `- ai state
```

Việc chia nhỏ state là một quyết định thiết kế cốt lõi, không chỉ là cách tổ chức
dữ liệu cho tiện.

### 4.4 Channel

`Channel` là boundary logic bên trong room, đồng thời là boundary cho
subscription.

Trong tài liệu này, channel gắn trực tiếp với ý tưởng chia room thành nhiều state
con nhỏ hơn.

Mỗi channel thường đại diện cho một state con hoặc một nhóm state con gần nhau về
ngữ nghĩa, và có thể có subscribe policy riêng dựa trên:

- member role
- loại member hoặc capability
- trạng thái hiện tại của room
- policy nghiệp vụ của chính state đó

Vì vậy, channel có hai vai trò chính:

- phân tách từng phần state hoặc concern trong room
- cho phép selective subscription và policy riêng trên từng phần

Ví dụ:

```text
RoomState
 |- participants
 |- chat
 |- presence
 |- game
 `- ai
```

Lưu ý:

- execution boundary chính vẫn là `room`
- channel không thay thế room như một runtime boundary độc lập
- channel là boundary để đọc, publish, và subscribe vào từng state con cần thiết

### 4.5 Mutation

`Mutation` là input có chủ đích từ client hoặc actor nhằm yêu cầu thay đổi room.

Mutation thường mang theo:

- room target
- channel target nếu có
- action
- payload
- auth context

Có thể mô tả tối thiểu như sau:

```text
Mutation {
  workspace_id
  room_id
  channel
  action
  payload
  auth_context
}
```

Mutation là `intent`, chưa phải `room event`.

### 4.6 Room event

`Room event` là canonical state-change record của room.

Trong kiến trúc này:

- room event được sinh tự động khi room logic mutate state hợp lệ
- room event đồng thời phục vụ record, replay, restore, replication và fanout
- room event là hình thức execution history chính của room

Cần phân biệt rõ:

- `mutation` = yêu cầu thay đổi
- `room event` = thay đổi state đã được chấp nhận và record
- `update` = dữ liệu được gửi tới subscriber

### 4.7 Update và derived view

Client không nhất thiết phải nhận toàn bộ room state.

Vì room được chia thành nhiều state con, mỗi client có thể chủ động quyết định nó
cần subscribe vào phần nào của room.

Điều này có nghĩa:

- client tự chọn state con hoặc channel mà nó cần quan tâm
- hệ thống chỉ publish các update thuộc phần đã subscribe và được phép đọc
- các subscribe policy được áp dụng riêng trên từng state con hoặc channel

Hệ thống có thể publish:

- patch
- snapshot cục bộ
- derived view theo role, channel, state, hoặc policy

Điều này cho phép tối ưu băng thông và giảm coupling giữa canonical state và UI
view.

Tóm lại, bản chất của thiết kế này là:

- room được chia thành nhiều state nhỏ
- mỗi state nhỏ có subscribe policy riêng
- client tự quyết định cần subscribe vào các state nhỏ nào
- publish layer chỉ gửi những gì thực sự liên quan đến client đó

---

## 5. Mô hình record, replay, restore và consistency

### 5.1 Tư duy cốt lõi

Room được xem là một `replicated realtime state machine`.

Room state thay đổi theo chuỗi `room events`, trong đó room event được sinh tự
động từ các thay đổi state được thực hiện trong mutation handling.

Có thể tóm tắt như sau:

```text
accepted mutation
  -> mutate room state
  -> auto capture state changes
  -> room events
  -> persist + replicate + publish
```

### 5.2 Room event log là canonical history

Room event log là nguồn chính để:

- record tiến hóa của room
- replay lại room state
- restore room khi runtime bị mất
- di chuyển room sang node khác
- hỗ trợ scale-out và recovery

Tài liệu này chốt rõ ràng:

- `room event log` mới là execution history chính
- `snapshot` chỉ là optimization cho recovery

### 5.3 Replay

`Replay` có nghĩa là tái dựng lại room state hoặc tái hiện quá trình tiến hóa
của room từ danh sách room events và snapshot nếu cần.

Replay phục vụ:

- debugging
- temporal inspection
- kiểm chứng behavior
- xây lại room state tại một mốc logic nào đó

ASCII flow:

```text
snapshot(optional)
   +
remaining room events
   |
   v
reconstructed room state
```

### 5.4 Restore

`Restore` là khả năng phục hồi room để tiếp tục vận hành sau các tình huống như:

- server restart
- process crash
- node failure
- room được move sang server khác

Restore được thực hiện bằng cách nạp snapshot phù hợp và apply tiếp room events
còn thiếu.

```text
load snapshot
   -> apply missing room events
   -> rebuild room state
   -> resume serving room
```

Restore là capability bắt buộc của runtime, không phải thao tác thủ công ngoài
hệ thống.

### 5.5 Determinism

Room logic phải `deterministic` theo nghĩa sau:

- với cùng room state và cùng input hợp lệ, kết quả logic phải giống nhau
- room state phải có thể khôi phục chính xác từ `snapshot + room event history`

Hệ quả kiến trúc của yêu cầu này là:

- dev buộc phải implement room logic trên loại state có thể sync, record,
  replay, và restore
- mọi thay đổi có ý nghĩa đến canonical room behavior phải đi qua state engine
  đó

### 5.6 Concurrent writes và hội tụ

Hệ thống được thiết kế cho true multi-writer, nên có thể xuất hiện các thay đổi
đồng thời.

Vì vậy canonical state engine phải:

- merge được concurrent changes hợp lệ
- đảm bảo eventual convergence cho phần state được chia sẻ
- giữ đủ metadata cần thiết cho sync và replay

Tài liệu này không chốt thuật toán merge cụ thể, nhưng bắt buộc state engine
phải đáp ứng được bài toán này.

### 5.7 Quan hệ giữa room event và audit log

`Room event` và `audit log` là hai lớp dữ liệu khác nhau:

- room event dùng cho thực thi, replay, restore, sync
- audit log dùng cho truy vết hệ thống và nghiệp vụ

Audit log có thể được sinh từ mutation, room event, hoặc hành vi vận hành, nhưng
không được xem là canonical execution history của room.

---

## 6. Mô hình tương tác

### 6.1 Nguyên tắc trải nghiệm lập trình room

Mục tiêu của kiến trúc là làm cho việc phát triển room realtime trở nên đơn
giản:

- dev tập trung vào validate và mutate state
- platform tự record thay đổi
- platform tự sync và publish update tới những bên liên quan

Mental model mong muốn:

```text
developer writes room logic as:

1. validate input
2. check auth and policy
3. mutate room state

platform handles:

4. detect state changes
5. create room events
6. persist and replicate
7. fanout updates
```

### 6.2 Luồng xử lý mutation

Luồng baseline:

```text
client mutation
   -> gateway auth + routing
   -> room runtime validation + fine-grained authz
   -> direct room state mutation
   -> automatic room event generation
   -> persist / replicate / publish
```

Ý nghĩa của luồng này:

- `mutation` là input có chủ đích
- room logic mutate state trực tiếp
- event sourcing của room được tự động hóa bởi state engine

### 6.3 Subscription

Client tự quyết định channel hoặc state con nào của room mà nó muốn subscribe.

Hệ thống duy trì ánh xạ logic:

```text
state/channel/view -> subscribers
```

Khi room event được ghi nhận, hệ thống xác định:

- phần state nào bị ảnh hưởng
- channel nào liên quan
- subscriber nào đã đăng ký phần đó
- subscriber nào thực sự có quyền nhận update theo policy tương ứng

Chỉ những update liên quan mới được gửi đi.

### 6.4 Update shape

Update gửi tới client có thể ở nhiều dạng:

- delta/patch
- partial snapshot
- derived view

Lựa chọn cụ thể sẽ do system design và implementation quyết định, nhưng summary
design chốt rằng publish layer phải tồn tại độc lập với mutation input layer.

---

## 7. Kiến trúc runtime cấp cao

### 7.1 Phân tách control plane và data plane

```text
+-------------------+        +--------------------------------+
|   Control Plane   |        |           Data Plane           |
|-------------------|        |--------------------------------|
| Workspace mgmt    |        | Gateway                        |
| Admins            |        | Room runtime                   |
| Tokens / Policies |        | Room event store               |
| Room metadata     |        | Snapshot store                 |
| Quotas            |        | Subscription / fanout layer    |
+-------------------+        +--------------------------------+
```

### 7.2 Thành phần chính

#### Gateway

Gateway là lớp ingress stateless, chịu trách nhiệm:

- nhận kết nối client
- authenticate
- route đến room phù hợp
- áp dụng coarse-grained authorization

#### Room runtime

Room runtime là nơi thực thi room logic.

Nó chịu trách nhiệm:

- tải room state vào memory hoặc runtime context phù hợp
- xử lý mutation
- mutate state
- sinh room event tự động thông qua state engine
- kích hoạt publish flow và persistence flow

#### Room event store

Room event store lưu canonical room event history.

Nó là nền tảng cho:

- replay
- restore
- migration room giữa nodes
- temporal debugging
- fault recovery

#### Snapshot store

Snapshot store lưu bản chụp state định kỳ hoặc theo chính sách phù hợp để rút
ngắn thời gian restore.

#### Subscription / fanout layer

Lớp này chịu trách nhiệm:

- xác định ai cần nhận update
- đồng bộ update qua node khi cần
- tối ưu fanout theo channel và policy

### 7.3 Hình dung tổng thể

```text
client
  |
  v
gateway
  |
  v
room runtime
  |
  +--> mutate platform-managed state
  |
  +--> auto generate room events
  |       |
  |       +--> room event store
  |       +--> snapshot store (periodic/policy based)
  |       `--> subscription fanout
  |
  `--> updates to interested subscribers
```

---

## 8. Boundary vận hành và ràng buộc cho implementation

### 8.1 Hard rule cho room developer

Room implementations bắt buộc phải sử dụng `platform-managed state`.

Điều này có nghĩa:

- room logic chỉ được mutate state mà platform có thể sync
- state đó phải được record, replay, và restore
- state không nằm trong cơ chế này không được ảnh hưởng đến canonical room
  behavior

Đây là ràng buộc có ý thức của kiến trúc, không phải recommendation mềm.

### 8.2 Ràng buộc về deterministic behavior

Room logic không được dựa vào những nguồn gây mất determinism cho canonical room
behavior, ví dụ:

- randomness không được record
- system time không được record
- side effect ngoài hệ thống ảnh hưởng trực tiếp tới state mà không đi qua event
  record

Nếu cần dùng các input như thời gian, random seed, hoặc external result, chúng
phải được đưa vào dưới dạng input đã được chấp nhận và record hợp lệ.

### 8.3 Boundary auth và policy

- `Gateway` xử lý authentication và coarse-grained checks
- `Room runtime` xử lý fine-grained authorization theo room và channel
- policy read/write có thể khác nhau theo từng channel hoặc từng view

### 8.4 Failure là tình huống kiến trúc mặc định

Hệ thống phải được thiết kế với giả định failure là bình thường:

- process có thể restart
- node có thể crash
- room có thể bị move sang node khác

Vì vậy `record`, `replay`, và `restore` không phải feature phụ, mà là một phần
của execution model.

---

## 9. MVP baseline và các quyết định để mở

### 9.1 MVP baseline

Phiên bản đầu tiên cần giữ được các nguyên lý sau:

- workspace và token management
- gateway cho API và realtime connections
- room runtime có khả năng xử lý mutation và mutate state trực tiếp
- room event log cho record và replay
- snapshot để restore nhanh hơn
- channel-based subscription
- state engine CRDT-compatible để xác thực mô hình multi-writer và convergence

Điều quan trọng là MVP có thể còn hạn chế ở topology hoặc deployment, nhưng
không được mâu thuẫn với các cam kết kiến trúc nền tảng.

### 9.2 Các quyết định để mở cho giai đoạn sau

Những điểm sau chưa được chốt trong tài liệu này:

- protocol cụ thể giữa client và gateway
- event encoding và wire format tối ưu
- topology fanout cuối cùng cho multi-node
- placement strategy và sharding strategy chi tiết
- conflict policy và state modeling chi tiết cho từng room type
- backend CRDT cụ thể trong dài hạn

---

## 10. Rủi ro và trade-off

| Quyết định                              | Lợi ích                                   | Chi phí / Rủi ro                            |
| --------------------------------------- | ----------------------------------------- | ------------------------------------------- |
| Room event sinh tự động từ state change | DX đơn giản, room dev dễ làm việc         | Cần state engine và abstraction đủ mạnh     |
| CRDT-compatible state engine            | Hỗ trợ multi-writer và convergence        | Modeling và debug phức tạp hơn state thường |
| Room event log là canonical history     | Replay, restore, migration, debugging tốt | Tốn chi phí lưu trữ và quản lý history      |
| Snapshot + replay                       | Recovery nhanh hơn                        | Tăng độ phức tạp vận hành                   |
| Channel-based subscription              | Giảm fanout thừa, tối ưu update           | Cần boundary state và policy rõ ràng        |
| Tách audit log khỏi room event          | Rành mạch về mục đích dữ liệu             | Cần pipeline record rõ ràng hơn             |

---

## 11. Tổng kết

Hệ thống này được định nghĩa như sau:

> Một nền tảng room realtime đa tenant, trong đó room logic làm việc trực tiếp
> trên state do platform quản lý; mọi thay đổi state hợp lệ được record tự động
> thành room events để phục vụ sync, replay, restore, và publish realtime.

Nó kết hợp các ý tưởng nền tảng sau:

- room-centric execution
- true multi-writer by design
- CRDT-compatible state engine
- automatic state-change recording
- replay và restore như capability kiến trúc mặc định
- selective subscription theo channel

Summary design này tạo ra khung quyết định để bước tiếp theo có thể đi vào:

- system design chi tiết
- skeleton hóa các thành phần
- coding theo một mô hình nhất quán
