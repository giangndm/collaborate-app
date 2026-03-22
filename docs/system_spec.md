# Đặc tả thiết kế hệ thống

## Nền tảng room realtime đa tenant với Console và Gateway tách biệt

---

## 1. Mục đích tài liệu

Tài liệu này mô tả `system design` cho nền tảng room realtime đã được chốt ở mức
`summary design` trong `docs/summary_design.md`.

Mục tiêu của tài liệu này:

- chuyển các quyết định kiến trúc cấp cao thành mô hình hệ thống cụ thể hơn
- làm nền cho bước `skeleton` và `coding`
- chốt boundary giữa các service, storage, trust model, và runtime flow

Tài liệu này chưa đi vào:

- module breakdown chi tiết bên trong từng service
- schema SQL/Redis cụ thể đến cấp bảng và key cuối cùng
- protocol frame schema đầy đủ đến từng field
- deployment topology production cuối cùng

---

## 2. Mục tiêu hệ thống

Hệ thống cần giải quyết đồng thời các bài toán sau:

- quản lý `workspace` đa tenant ở control plane
- vận hành `room realtime` ở data plane
- cho phép người dùng kết nối qua gateway phù hợp nhất theo điều kiện mạng
- cho phép một room có thể chạy trên nhiều gateway cùng lúc
- giữ được khả năng `record`, `replay`, `restore`, và `replicate`
- giữ DX đơn giản cho developer room: chủ yếu là validate rồi mutate state

Các ràng buộc chính:

- `Console` không cần scale mạnh
- `Gateway` phải scale ngang tốt
- `Workspace-level data` thuộc SQL
- `Room-level data` thuộc Redis
- room state phải CRDT-compatible
- room event phải được sinh tự động từ state change

---

## 3. Bối cảnh hệ thống và boundary 2 service

Hệ thống được chia thành 2 service chính:

- `Console service`
- `Gateway service`

### 3.1 Console service

`Console` là control plane.

Nó chịu trách nhiệm:

- admin login
- quản lý workspace
- quản lý workspace secret và API credential
- quản lý room policy / metadata ở mức workspace
- lưu trữ dữ liệu bền vững trong SQL
- xuất dữ liệu workspace tối thiểu để gateway đồng bộ định kỳ

`Console` dùng:

- `Clerk` cho admin authentication
- `SQL` là source of truth
- `SeaORM` để giữ DB-agnostic
- `Refine.dev` cho admin frontend

### 3.2 Gateway service

`Gateway` là cửa ngõ để đi vào hệ thống từ phía client hoặc server của bên sử
dụng.

Nó chịu trách nhiệm:

- cung cấp API sinh `join token` và `guest token`
- nhận WebSocket connection từ end-user
- xác thực room access token
- chạy room runtime
- xử lý mutation và subscription realtime
- record, replay, restore, replicate room state
- serve built artifact của room frontend trong bản release

`Gateway` dùng:

- `Redis` là room execution backbone
- `React` cho room frontend, nhưng source code đặt trong `frontends/`
- built artifact của room frontend được gateway serve ở bản release

### 3.3 ASCII tổng thể

```text
         +---------------------------+
         |  Admin Console Frontend   |
         |    Refine.dev build       |
         +------------+--------------+
                      |
                      v
         +---------------------------+
         |      Console Service      |
         |---------------------------|
         | Clerk auth                |
         | Workspace management      |
         | Workspace credentials     |
         | Room metadata / policy    |
         | SQL via SeaORM            |
         +------------+--------------+
                      |
                      | periodic pull
                      v
+--------------------------------------------------+
|                  Gateway Service                 |
|--------------------------------------------------|
| Public API for customer servers                  |
| WebSocket for room clients                       |
| Token generation and token verification          |
| Room runtime / mutation / subscription           |
| Room frontend static delivery                    |
| Redis-backed record / replay / restore           |
+---------------------------+----------------------+
                            |
                            v
                       +----------+
                       |  Redis   |
                       +----------+
```

---

## 4. Kiến trúc backend: Rust hexagonal design

Cả `Console` và `Gateway` đều được xây theo `Rust hexagonal architecture`.

Điều này có nghĩa:

- domain và application logic không phụ thuộc trực tiếp vào HTTP, WebSocket, SQL,
  Redis, Clerk, Refine, hay React
- mọi tích hợp hạ tầng đi qua ports và adapters
- inbound adapters nhận request hoặc message từ bên ngoài
- outbound adapters nói chuyện với DB, Redis, hoặc service khác

Lợi ích của hướng này:

- tách biệt business rules khỏi framework
- dễ test hơn ở mức use case và domain
- dễ thay adapter hạ tầng khi cần
- giữ ranh giới rõ giữa control plane và data plane

Lưu ý: tài liệu này chưa chốt module chi tiết bên trong từng service; phần đó sẽ
thảo luận sau.

---

## 5. Data ownership và storage boundary

### 5.1 SQL cho workspace-level

`Console` là nơi sở hữu dữ liệu control plane trong SQL.

Nhóm dữ liệu điển hình:

- workspace
- admin / operator mapping
- workspace token / secret / API credentials
- policy ở mức workspace
- room catalog metadata
- cấu hình hệ thống dài hạn

`Gateway` không được coi SQL là hot path dependency.

### 5.2 Redis cho room-level

`Gateway` sở hữu room-level execution data trên Redis.

Nhóm dữ liệu điển hình:

- room event queue
- room notify channel
- snapshot hoặc checkpoint liên quan đến room
- dữ liệu phục vụ replay / restore / replication

Trong thiết kế này:

- Redis là room execution backbone
- Redis vừa phục vụ record vừa phục vụ replication trigger
- room runtime có thể được dựng lại từ dữ liệu room-level trong Redis

### 5.3 Boundary chốt

```text
Workspace level  -> SQL via Console
Room level       -> Redis via Gateway
```

---

## 6. Trust model và credential model

Hệ thống có 3 loại credential chính với vai trò khác nhau.

### 6.1 Clerk session

Dùng cho admin/operator đăng nhập vào `Console`.

Phạm vi:

- chỉ áp dụng cho control plane
- không dùng để join room
- không kéo vào hot path của gateway

### 6.2 Workspace API key / secret

Đây là credential để phía server của khách hàng gọi `Gateway API` nhằm sinh token
truy cập room.

Phạm vi:

- server-to-server
- privileged API ở gateway
- không dùng trực tiếp để end-user join room

### 6.3 Join token / guest token

Đây là credential để end-user vào room qua gateway.

- `join token`: gắn với member payload cụ thể như `name`, `email`, `ext_id`
- `guest token`: linh hoạt hơn theo policy cho phép

Token được sinh từ `workspace secret` và payload người dùng.

Trong baseline hiện tại, token nên mang tối thiểu các thông tin sau:

- `workspace_id`
- `room_id` hoặc room scope nếu cần giới hạn phạm vi
- `member payload`
- `exp` hoặc thời gian hết hạn
- `kid` hoặc key version

Gateway sẽ dùng token này để:

- xác thực kết nối WebSocket
- dựng `member context`
- áp policy join room

### 6.4 Workspace secret sync

`Gateway` không đồng bộ raw secret riêng cho từng workspace từ `Console`.

Thay vào đó, hệ thống sử dụng một `fixed system secret` dùng chung cho toàn hệ
thống để phục vụ việc ký và verify các room access token ở gateway.

`Console` chỉ đồng bộ xuống gateway các dữ liệu workspace tối thiểu như:

- `workspace_id`
- `workspace_status`
- metadata credential cần thiết
- token policy
- room-level policy summary

Ý nghĩa của quyết định này:

- giảm độ phức tạp của secret sync giữa `Console` và `Gateway`
- giữ `Gateway` tự chủ trong hot path khi sinh và verify token
- tránh phải quản lý rotation riêng cho secret của từng workspace trong baseline

Trade-off:

- fixed system secret là trust root chung của toàn hệ thống
- cần được bảo vệ chặt ở mức vận hành và triển khai
- thay đổi hoặc rotate secret này là sự kiện cấp hệ thống, không phải cấp workspace

Để giảm blast radius trong khuôn khổ thiết kế đơn giản hiện tại, baseline vẫn yêu
cầu:

- token phải mang `kid` hoặc version
- hệ thống phải hỗ trợ emergency rotation ở cấp toàn hệ thống
- token nên có TTL ngắn
- gateway chỉ giữ secret trong runtime config hoặc secret manager phù hợp, không
  ghi lộ ra logs

---

## 7. Workspace sync model: Console -> Gateway

### 7.1 Nguyên tắc

Gateway không được phụ thuộc trực tiếp vào Console trong hot path của room.

Thay vào đó, Gateway định kỳ pull dữ liệu workspace cần thiết từ Console.

### 7.2 Sync strategy

Chiến lược baseline:

- `periodic pull only`
- `incremental sync` dựa trên trường `last_updated` hoặc tương đương
- cấu hình qua `ENV` hoặc `CLI args`
- không có push invalidation
- không có manual refresh trong baseline

### 7.3 Dữ liệu sync tối thiểu

Gateway chỉ đồng bộ phần thật sự cần cho room-level execution:

- `workspace_id`
- `workspace_status`
- `workspace API credential metadata`
- `token policy`
- `room-level policy summary`
- `last_updated` hoặc marker tương đương cho incremental sync

Không đồng bộ các dữ liệu control plane không cần thiết như toàn bộ admin data hay
toàn bộ user data.

### 7.4 Hệ quả vận hành

- Console bị lỗi không làm hỏng ngay room hot path nếu gateway đã sync được dữ
  liệu cần thiết
- thay đổi secret hoặc policy ở Console sẽ có độ trễ áp dụng tương ứng với
  `sync interval`

ASCII flow:

```text
Console SQL
   -> Console export API
   -> Gateway periodic sync
   -> Gateway workspace cache
```

---

## 8. API surface ở mức hệ thống

### 8.1 Console API

Console cung cấp API cho control plane:

- quản lý workspace
- quản lý workspace credentials
- quản lý room metadata / policy ở mức workspace
- export workspace sync payload cho gateway

### 8.2 Gateway API cho phía server của khách hàng

Gateway cung cấp public API để phía server của khách hàng sử dụng `workspace API
key / secret` gọi vào.

Các API baseline:

- sinh `join token`
- sinh `guest token`

### 8.3 Gateway WebSocket cho end-user

End-user kết nối trực tiếp vào gateway qua WebSocket để làm việc với room.

Toàn bộ interaction realtime baseline đi qua một kết nối WebSocket duy nhất.

---

## 9. Realtime interaction model qua WebSocket

### 9.1 Xác thực kết nối

WebSocket sử dụng `query param` để xác thực:

```text
ws://gateway/...?...&token=<join_or_guest_token>
```

Trong production, toàn bộ API và WebSocket bắt buộc phải chạy trên `TLS`.

Nói cách khác:

- HTTP phải là `https`
- WebSocket phải là `wss`

Vì token nằm trên query param, mọi lớp gateway, reverse proxy, access log,
application log, tracing, và telemetry bắt buộc phải `redact` giá trị `token`.

Baseline hiện tại vẫn chấp nhận `?token=` để đơn giản hóa client integration,
nhưng đi kèm các ràng buộc sau:

- token phải có TTL ngắn
- token nên có scope rõ theo workspace và room
- token không được log thô ở bất kỳ lớp nào

Gateway verify token ngay ở handshake hoặc ngay khi thiết lập session.

Nếu token hợp lệ, gateway dựng:

- `workspace context`
- `room access context`
- `member context`

### 9.2 Mô hình giao tiếp: RPC + event

Sau khi kết nối thành công, WebSocket hoạt động theo mô hình `RPC + event`.

Client gửi các message dạng lệnh, ví dụ:

- `subscribe`
- `unsubscribe`
- `mutation`
- `ping`

Với `mutation`, client phải gửi kèm `mutation_id` duy nhất trong phạm vi session
hoặc theo policy idempotency phù hợp.

Gateway trả về:

- `rpc_result`
- `rpc_error`
- `event/update`

### 9.3 Mutation và subscription cùng đi qua WebSocket

Thiết kế baseline chốt rằng:

- `mutation` đi qua WebSocket
- `subscription` đi qua WebSocket
- `update/event` cũng đi qua cùng session WebSocket đó

Điều này giúp:

- gom auth context, member context, và subscribe context vào một session thống
  nhất
- đơn giản hóa API surface
- thuận tiện hơn cho room lifecycle và presence

ASCII flow:

```text
client websocket session
  -> auth by ?token=
  -> rpc: subscribe
  -> rpc: mutation
  <- rpc_result / rpc_error
  <- event: room updates
```

---

## 10. Mô hình room state và subscription

### 10.1 Room state là tập các state con

Room state không được xem như một khối lớn duy nhất.

Bản chất của room state là tập hợp nhiều `state con`, ví dụ:

```text
Room State
 |- participant state
 |- chat state
 |- presence state
 |- voting state
 `- game state
```

### 10.2 Client tự quyết định subscribe phần cần thiết

Mỗi client có thể quyết định nó cần subscribe vào state con nào hoặc channel nào
trong room.

Điều này cho phép:

- tối ưu băng thông
- giảm dữ liệu không cần thiết gửi xuống client
- áp policy đọc khác nhau theo từng phần state

### 10.3 Subscribe policy riêng theo state nhỏ

Mỗi state con hoặc channel có thể có subscribe policy riêng dựa trên:

- member role
- capability
- trạng thái room
- policy nghiệp vụ của state đó

Tóm lại, bản chất thiết kế là:

- room được chia thành nhiều state nhỏ
- mỗi state nhỏ có subscribe policy riêng
- client tự quyết định subscribe phần mình cần

---

## 11. Distributed room runtime model

### 11.1 Mục tiêu thực tế của mô hình multi-gateway

Mục tiêu chính của mô hình này không chỉ là scale compute, mà là giải bài toán
`network reachability`.

Trong thực tế, người dùng ở các nhà mạng hoặc điều kiện mạng khác nhau có thể kết
nối tốt với các gateway khác nhau. Vì vậy:

- mỗi người dùng nên được kết nối vào `gateway tốt nhất`
- room không bị ép chạy duy nhất ở một gateway cố định trong mọi trường hợp

### 11.2 Room có thể chạy trên nhiều gateway cùng lúc

Một room có thể có local runtime trên nhiều gateway cùng lúc.

Đây là mô hình `multi-master room runtime`.

Hai mode vận hành có thể cùng tồn tại về mặt chính sách:

- `tiết kiệm`: cố gắng chỉ một gateway chạy room
- `locality-first`: user vào gateway nào thì gateway đó có thể chạy local runtime

System design baseline ưu tiên mô hình `locality-first`.

### 11.3 Room lifecycle trên từng gateway

Một gateway sẽ tạo local room runtime khi có local demand, ví dụ:

- user local join room
- room cần được phục vụ trên gateway đó

Gateway không cần giữ local runtime mãi mãi.

### 11.4 Grace period eviction

Khi số local participant của room trên một gateway về `0`:

- gateway bắt đầu `grace period timer`
- nếu có user quay lại trong khoảng này thì hủy eviction
- nếu hết grace period mà vẫn không có local demand thì xóa local runtime khỏi
  memory

Việc evict local runtime không làm mất canonical room history vì room event log và
snapshot vẫn nằm ở Redis.

ASCII flow:

```text
local participant count > 0
   -> keep local room runtime alive

local participant count == 0
   -> start grace timer
   -> if rejoin before timeout: cancel eviction
   -> else: evict local runtime
```

---

## 12. Redis room event store model

### 12.1 Mỗi room có một queue riêng

Trong Redis, mỗi room có một `event queue` riêng.

Queue này là nguồn chính cho:

- record
- replay
- restore
- replication catch-up

### 12.2 Mỗi room có một notify channel riêng

Ngoài queue, mỗi room có một `notify channel` theo `room_id`.

Khi có event mới:

- event được append vào queue của room
- đồng thời có một notify event được fire lên channel của room

### 12.3 Queue là nguồn chuẩn, channel chỉ là trigger

Đây là điểm rất quan trọng của thiết kế:

- `queue` là canonical event history ở room level
- `channel` chỉ là tín hiệu có event mới
- gateway khác khi nhận notify sẽ kéo event từ queue về

Convergence của hệ thống không được định nghĩa bởi thứ tự notify hoặc bởi một cơ
chế ordering trung tâm riêng. Nó dựa chủ yếu vào `CRDT merge semantics` của state
engine.

Như vậy:

- pub/sub không phải persistence layer
- nếu một gateway bị chậm hoặc miss notify, nó vẫn có thể catch up bằng queue

Baseline catch-up strategy khi miss notify là:

- ngoài việc lắng nghe notify channel, mỗi gateway sẽ tự poll room queue định kỳ
  `5 giây` một lần theo mặc định
- polling này chỉ áp dụng cho các `room local active`, tức là room hiện đang có
  local runtime hoặc đang trong grace period eviction
- polling định kỳ này là lớp an toàn để bù cho notify bị miss hoặc bị trễ

### 12.4 Gateway lắng nghe room channel

Những gateway đang cầm local runtime của room sẽ subscribe vào channel của room.

Khi nhận notify:

- nếu notify là do chính gateway đó phát ra thì bỏ qua
- nếu notify từ gateway khác thì kéo các event chưa thấy từ queue về để apply

Để làm được việc này, mỗi event cần mang ít nhất:

- `room_id`
- `event_id` hoặc offset tương đương
- `origin_gateway_id`

### 12.5 Gateway cần theo dõi offset

Mỗi local room runtime cần giữ:

- `last_applied_event_id` hoặc offset tương đương

Đây là cơ sở để:

- kéo incremental events từ queue
- tránh apply trùng
- catch up sau khi miss notify

`event_id` hoặc offset ở đây chủ yếu dùng cho việc theo dõi log position, incremental
pull, và duplicate suppression ở mức runtime. Việc hội tụ trạng thái cuối cùng vẫn
dựa chủ yếu vào CRDT merge.

Chi tiết contract của `event cursor`, bao gồm cách sinh `event_id/offset`, định
nghĩa chính xác của `unseen events`, và rule dedupe/apply cuối cùng, được để lại
cho bước thiết kế chi tiết tiếp theo.

ASCII flow:

```text
Gateway A
  -> append event E vào room queue
  -> publish notify(room_id, event_id, origin=A)

Gateway B đang cầm room
  -> nhận notify
  -> pull các event chưa thấy từ room queue
  -> apply vào local runtime
```

---

## 13. Room event model và atomic change model

### 13.1 Room event là state-change record sinh tự động

Room event không được mô hình hóa chủ yếu như business event thủ công.

Trong thiết kế này:

- developer chủ yếu validate rồi mutate state
- platform tự phát hiện thay đổi state
- platform tự sinh `room event` từ thay đổi đó

### 13.2 Baseline atomic primitives

Mô hình event của room phải hạ được về các atomic change primitives baseline sau:

- `set`
- `del`
- `inc`
- `dec`

Đây là baseline bắt buộc của system design hiện tại.

### 13.3 Ý nghĩa của các primitive atomic

- `set`: gán hoặc cập nhật giá trị
- `del`: xóa một phần state
- `inc`: tăng giá trị đếm theo kiểu atomic
- `dec`: giảm giá trị đếm theo kiểu atomic

`inc` và `dec` là primitive đặc biệt quan trọng để xử lý các bài toán concurrent
như:

- voting
- score
- counter
- quota
- presence counter

Mục tiêu là tránh việc developer phải tự mô hình hóa các thao tác đếm theo kiểu
read-modify-write dễ xung đột.

### 13.4 Quan hệ với CRDT-compatible state engine

Các primitive atomic này phải tương thích với state engine CRDT-compatible.

Điều này giúp:

- local-first apply vẫn hội tụ được khi có concurrent writes
- replication giữa nhiều gateway không làm diverge state
- replay và restore giữ đúng semantics của state change

Tuy nhiên, `set`, `del`, `inc`, `dec` chỉ an toàn khi từng phần state được gắn với
loại dữ liệu CRDT phù hợp. Baseline system design chốt mapping tối thiểu như sau:

- counter hoặc voting count -> dùng counter semantics tương thích `inc/dec`
- register đơn giản -> dùng register semantics phù hợp cho `set`
- map hoặc object state -> dùng map semantics có thể replay từng field change
- delete -> phải có delete semantics nhất quán với replay và merge, không được coi
  đơn thuần là xóa mù không có quy tắc

Những state type không ánh xạ rõ được vào các semantics trên không nên được đưa
vào multi-master room baseline.

---

## 14. Local-first apply và replication model

### 14.1 Nguyên tắc local-first

Khi một user gửi mutation vào gateway local:

- gateway local xử lý mutation ngay
- room logic validate rồi mutate state local ngay tại chỗ
- room event được sinh tự động từ local state change

Đây là quyết định có chủ ý để tối ưu DX và latency.

### 14.2 Replication flow

Sau khi local state change được sinh ra:

- gateway append room event vào Redis queue
- gateway fire notify lên room channel
- các gateway khác đang cầm room sẽ kéo event từ queue về
- các gateway đó apply event vào local runtime của mình

ASCII flow:

```text
local client
   -> local gateway room runtime
   -> validate + mutate state locally
   -> auto generate room event
   -> append to Redis room queue
   -> publish notify(room_id, origin_gateway, event_id)

other gateways holding same room
   -> receive notify
   -> pull unseen events from room queue
   -> apply remote events
```

### 14.3 CRDT giải quyết xung đột

Vì room state là CRDT-compatible nên khi nhiều gateway cùng phát sinh thay đổi
đồng thời:

- mỗi gateway vẫn có thể apply local trước
- conflict được hấp thụ ở level state engine
- hệ thống hướng tới `eventual convergence`

System design này chốt rõ rằng `multi-master ordering/convergence` dựa chủ yếu vào
`CRDT merge`, không dựa vào một central sequencer riêng.

Redis queue ở đây là shared record/replay/restore backbone, không phải central
authority buộc mọi mutation phải round-trip trước mới có hiệu lực local.

---

## 15. Replay và restore model

### 15.1 Replay

`Replay` dùng để tái dựng room state hoặc tái hiện quá trình tiến hóa của room từ
room event history.

Replay phục vụ:

- debugging
- temporal inspection
- rebuild state
- sync catch-up

### 15.2 Restore

`Restore` dùng khi gateway cần khôi phục local room runtime sau các tình huống như:

- gateway restart
- process crash
- room được cầm lại bởi một gateway khác
- local runtime đã bị evict trước đó và cần dựng lại

### 15.3 Snapshot + queue

Restore nên tận dụng:

- `snapshot` nếu có
- sau đó apply tiếp event còn thiếu từ room queue

ASCII flow:

```text
load snapshot if available
   -> read missing room events from Redis queue
   -> rebuild local room state
   -> resume room service
```

---

## 16. Failure model và trade-offs

### 16.1 Best-effort durability after local apply

Thiết kế baseline chấp nhận mô hình:

- local room state được apply trước
- append vào Redis là bước tiếp theo với ưu tiên cao
- nếu append lỗi thì hệ thống `warn + auto-retry`
- không rollback local state

Mỗi local room runtime cần duy trì trạng thái `store_healthy` để phản ánh việc các
sự kiện local gần đây đã được đẩy thành công sang room event store hay chưa.

Có thể hiểu đơn giản:

- `store_healthy = true`: local runtime đang đẩy sự kiện sang store thành công theo
  kỳ vọng hiện tại
- `store_healthy = false`: local runtime đang có vấn đề với việc flush sự kiện sang
  store, dù local apply vẫn có thể đã xảy ra

Ở mức RPC semantics, baseline chốt rằng:

- `RPC success` nghĩa là `local success`
- nó không đồng nghĩa với việc event đã được persist thành công vào Redis

Để tránh mutation bị xử lý lặp ngoài ý muốn, gateway phải hỗ trợ idempotency ở mức
`mutation_id` trong phạm vi phù hợp với session hoặc room runtime.

Đây là trade-off có chủ ý để ưu tiên local UX và network locality.

### 16.2 Khi append Redis lỗi

Khi append event vào Redis thất bại:

- hệ thống ghi warning/telemetry
- hệ thống auto-retry theo retry policy nội bộ
- local state đã apply không bị rollback
- local room runtime có thể chuyển sang trạng thái `store_healthy = false`

Điều này đồng nghĩa:

- local user có thể đã thấy thay đổi
- nhưng các gateway khác chưa thấy thay đổi đó cho đến khi event được flush thành
  công vào Redis

Khi việc flush quay lại bình thường, local runtime có thể đưa trạng thái về
`store_healthy = true`.

### 16.3 Rủi ro được chấp nhận

Nếu gateway chết trước khi flush event thành công vào Redis, một phần local-first
change chưa persist có thể bị mất.

Đây là rủi ro được chấp nhận trong baseline hiện tại.

### 16.4 Console failure

Nếu Console bị lỗi tạm thời:

- room hot path của gateway vẫn có thể hoạt động nếu workspace config cần thiết đã
  sync trước đó

### 16.5 Redis failure

Nếu Redis bị lỗi:

- room replication, replay, restore, và persistence backbone bị ảnh hưởng trực
  tiếp
- đây là dependency trọng yếu của data plane

### 16.6 Eventual convergence

Hệ thống dựa vào các điều kiện sau để đạt convergence:

- state engine CRDT-compatible
- deterministic room logic
- room event model dựa trên atomic operations
- incremental pull từ shared room queue

---

## 17. Frontend model

### 17.1 Console frontend

- source code đặt trong `frontends/`
- dùng `Refine.dev`
- phục vụ admin workflow của Console

### 17.2 Room frontend

- source code đặt trong `frontends/`
- dùng `React`
- build artifact được gateway serve ở bản release

### 17.3 Boundary frontend/backend

- source frontend không được nhúng trực tiếp vào source backend
- release artifact có thể được gateway serve như static assets

---

## 18. Các quyết định để mở

Những điểm sau chưa chốt ở tài liệu này:

- module breakdown chi tiết trong từng service
- schema Redis cuối cùng cho queue/channel/snapshot
- chi tiết format của WebSocket frames theo RPC + event
- chiến lược snapshot cụ thể
- placement heuristic chính xác cho việc chọn `gateway tốt nhất`
- cơ chế observability và metrics chi tiết
- deployment production topology chi tiết

---

## 19. Tổng kết

Hệ thống này được định nghĩa như sau:

> Một nền tảng room realtime đa tenant với hai service tách biệt: `Console` cho
> control plane trên SQL, và `Gateway` cho data plane trên Redis. Gateway là cửa
> ngõ chung cho client và customer server, hỗ trợ sinh room access token, chạy
> room runtime theo mô hình multi-master locality-first, record room event dưới
> dạng state-change primitives, và dùng Redis queue + channel để replicate,
> replay, và restore room state.

Các quyết định cốt lõi đã được chốt trong system design này:

- `Console` và `Gateway` là hai service riêng
- backend dùng `Rust hexagonal architecture`
- `Console` dùng `Clerk + SQL + SeaORM`
- `Gateway` dùng `Redis` cho room-level execution backbone
- `Workspace-level` và `Room-level` có storage ownership tách biệt
- WebSocket là kênh chính cho `mutation + subscription + event`
- gateway auth room bằng token ở query param `?token=`
- room runtime vận hành theo mô hình `multi-master locality-first`
- local room runtime dùng `grace period eviction`
- room event được sinh tự động từ state change
- baseline atomic change primitives là `set`, `del`, `inc`, `dec`
- local-first apply được ưu tiên, với best-effort durability sau đó

Tài liệu này đủ làm nền để đi sang bước tiếp theo:

- thiết kế skeleton theo hexagonal ports/adapters
- chốt contract giữa service và storage
- triển khai coding theo từng luồng chính
