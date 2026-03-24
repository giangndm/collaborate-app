# Thiết kế thư viện Syncable State

## 1. Mục tiêu

Thư viện `syncable-state` nhằm giải quyết bài toán đồng bộ state typed cho các ứng
dụng realtime trong repo này, theo hướng:

- developer làm việc với state typed thay vì JSON path và `get/set` thủ công
- mỗi mutation local tự động sinh delta có `path` chính xác
- nested state và nested container chỉ cần capture thay đổi của chính nó vào
  `ctx`, không cần tự lắp patch cho toàn bộ cây state
- state có thể chụp `snapshot` tại bất kỳ thời điểm hợp lệ nào
- state có thể phát `delta` theo từng mutation batch để gửi cho subscriber hoặc
  transport layer

Thư viện này không có mục tiêu giải bài toán transport, websocket protocol, hay
delivery guarantee ở tầng mạng. Nó chỉ sở hữu typed state, change capture,
snapshot, delta, và metadata cần thiết để layer trên có thể sync đúng.

## 2. Nguyên tắc kiến trúc

### 2.1 Typed-first

API phải hướng typed state làm trung tâm. Developer viết code theo domain model
thay vì làm việc trực tiếp với dynamic tree.

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

### 2.2 Explicit sync containers

Chỉ những field có kiểu `Syncable*` mới là primitive đồng bộ. Thư viện không có
mục tiêu biến mọi field Rust thường thành CRDT một cách ngầm định.

Điều này giúp:

- API rõ ràng
- merge semantics rõ ràng
- tài liệu dễ hiểu và dễ debug hơn
- macro không cần làm các phép "ma thuật" khó bảo trì

### 2.3 Capture changes qua context

Mỗi mutation typed nhận `&mut ChangeCtx` hoặc `&mut BatchTx`. Container sẽ tự:

- apply thay đổi vào internal state
- sinh `ChangeEnvelope`
- đẩy vào batch hiện tại
- commit batch để tạo `DeltaBatch` có `seq`

Ví dụ:

```rust
let mut batch = self.ctx.begin_batch()?;
let doc = self.state.docs.get_mut(&doc_id)?;
doc.title.set(&mut batch, title);
doc.revision.increment(&mut batch, 1);
let _ = batch.commit();
```

### 2.4 Path phải ổn định theo identity logic

Với collection có identity, path không được dựa trên index vật lý. Path phải dựa
trên identity business hoặc identity ổn định của item.

Ví dụ:

```text
docs[id="doc_123"].title
docs[id="doc_123"].content
```

Không dùng:

```text
docs[3].title
```

Vì index vật lý sẽ bị xáo trộn dưới concurrent insert, delete, reorder.

## 3. Phạm vi của thư viện

`syncable-state` sở hữu:

- typed syncable containers
- derive macro và schema metadata
- local mutation capture
- snapshot generation
- delta generation
- state sequence metadata
- remote apply và typed route theo path

`syncable-state` không sở hữu:

- websocket session management
- TCP framing
- subscriber persistence policy
- stream replay storage policy
- auth và subscribe permission
- transport retry strategy

Layer bên trên có thể dùng `snapshot + delta + seq` của thư viện để tự quản lý
replay và recover.

## 4. Core data model

### 4.1 ChangeCtx

`ChangeCtx` là nơi cấp phát sequence và thu gom delta local cho một state.

```rust
pub struct ChangeCtx {
    replica_id: ReplicaId,
    next_seq: u64,
    open_batch: Vec<ChangeEnvelope>,
    pending: Vec<DeltaBatch>,
}
```

Trách nhiệm chính:

- giữ `replica_id` của state/replica hiện tại
- cấp `seq` tăng đơn điệu cho từng `DeltaBatch` đã commit
- thu gom `ChangeEnvelope` vào batch hiện tại
- cho phép `poll()` hoặc `drain()` để transport lấy ra delta

Trong v1, `seq` là version chính cho subscribe, catch-up, và snapshot alignment.
`replica_id` được đưa vào thiết kế ngay từ đầu để mở đường cho replication phức
tạp hơn, nhưng v1 sẽ ưu tiên `single authoritative writer`.

Quan trọng: `seq` chỉ có một chủ sở hữu. Chỉ authoritative room/state stream
được phép cấp `seq`. Subscriber handler không được mint `seq` riêng. Handler chỉ
được lưu, checkpoint, đánh index, và replay lại `seq` đã nhận từ upstream state.

### 4.2 Batch contract

`seq` tăng theo từng mutation batch đã commit, không tăng theo từng
`ChangeEnvelope` con.

V1 chốt contract:

- một domain operation commit thành một `DeltaBatch`
- một `DeltaBatch` có thể chứa nhiều `ChangeEnvelope`
- `seq` tăng một đơn vị mỗi lần commit batch

API gợi ý:

```rust
pub struct BatchTx<'a> {
    ctx: &'a mut ChangeCtx,
}

impl ChangeCtx {
    pub fn begin_batch(&mut self) -> Result<BatchTx<'_>, SyncError>;
    pub fn poll(&mut self) -> Option<DeltaBatch>;
}

impl BatchTx<'_> {
    pub fn push(&mut self, change: ChangeEnvelope);
    pub fn commit(self) -> Option<DeltaBatch>;
}
```

Mục tiêu là tránh ambiguity kiểu một hàm domain có nhiều mutation nhỏ nhưng lại
không rõ phải phát một hay nhiều delta batch. Trong v1 hiện thực, `begin_batch()`
có thể fail nếu runtime đã bị khóa vào remote authoritative stream, còn
`commit()` trả `None` cho empty batch để không đốt `seq` vô ích.

### 4.3 Snapshot bundle

Snapshot luôn đi kèm sequence của state tại thời điểm chụp.

`seq` trong snapshot phải được lấy từ cùng authoritative runtime/context đang sở
hữu stream delta. Nói cách khác, `snapshot()` không được tự ý phát minh một mốc
sequence khác với mốc mà `ChangeCtx` đang cấp cho `DeltaBatch`.

```rust
pub struct SnapshotBundle<TSnapshot> {
    pub seq: u64,
    pub snapshot: TSnapshot,
}
```

Ý nghĩa:

- client/subscriber nhận snapshot sẽ biết mình đang đứng ở mốc `seq` nào
- batch tiếp theo sau snapshot phải có `from_seq == snapshot.seq`
- nếu thiếu event, transport layer có thể dựa vào `seq` để replay hoặc resnapshot

### 4.4 DeltaBatch

Thư viện không phát từng primitive op rời rạc mà phát theo `batch` của một lần
mutation đã commit.

```rust
pub struct DeltaBatch {
    pub replica_id: ReplicaId,
    pub from_seq: u64,
    pub to_seq: u64,
    pub changes: Vec<ChangeEnvelope>,
}
```

V1 sử dụng continuity rule rõ ràng:

- snapshot tại `seq = N` nghĩa là state đã bao gồm mọi batch đã commit đến `N`
- batch tiếp theo phải có `from_seq = N`
- sau khi apply batch, state chuyển sang `seq = to_seq`
- với authoritative writer v1, mỗi commit tăng một bước:
  `to_seq = from_seq + 1`
- với replay, batch sau phải thỏa `next.from_seq == prev.to_seq`

Mục đích:

- để transport lưu và replay theo thứ tự
- để client phát hiện gap event
- để room runtime broadcast theo đơn vị mutation có ý nghĩa

### 4.5 ChangeEnvelope

Mỗi thay đổi nested được biểu diễn thành một envelope có path và op typed.

```rust
pub struct ChangeEnvelope {
    pub path: SyncPath,
    pub op: ChangeOp,
}
```

Trong v1, `ChangeEnvelope` có thể giữ metadata tối thiểu để hệ thống gọn nhất.
Nếu cần true multi-writer sau này, có thể mở rộng thêm `op_id`, `deps`, hoặc
version vector mà không phá vỡ mental model hiện tại.

### 4.6 SyncPath

Path là logical path typed, không phải string patch ngẫu hứng.

```rust
pub enum PathSegment {
    Field(&'static str),
    Id(String),
    Key(String),
}

pub struct SyncPath(pub Vec<PathSegment>);
```

Path trả lời câu hỏi: thay đổi xảy ra ở đâu.

### 4.7 ChangeOp

`ChangeOp` là enum bọc các `*Op` con cho từng loại dữ liệu cụ thể.

```rust
pub enum ChangeOp {
    String(StringOp),
    Text(TextOp),
    List(ListOp),
    Counter(CounterOp),
    Map(MapOp),
    Struct(StructOp),
}
```

Nguyên tắc:

- `path` trả lời "đổi ở đâu"
- `ChangeOp` trả lời "đổi như thế nào"
- merge logic được tách theo module dữ liệu, không dồn vào một mega-enum phẳng

## 5. Syncable container set v1

### 5.1 SyncableString

Dùng cho scalar string, metadata ngắn, title, label.

```rust
pub enum StringOp {
    Set(String),
    Clear,
}
```

Semantics v1:

- typed register string
- ưu tiên đơn giản và dễ predict
- không tối ưu cho text edit theo ký tự

### 5.2 SyncableText

Dùng cho text cần chỉnh sửa incremental.

```rust
pub enum TextOp {
    Splice {
        index: usize,
        delete: usize,
        insert: String,
    },
    Clear,
}
```

Semantics v1:

- public API typed ở mức `splice`, `replace_all`, `clear`
- internal core có thể là sequence/text CRDT riêng
- transport và room logic chỉ cần thấy `TextOp`

### 5.3 SyncableVec<T>

Dùng cho ordered collection có identity ổn định.

```rust
pub enum ListOp {
    Insert {
        id: String,
        after: Option<String>,
        value: SnapshotValue,
    },
    Delete {
        id: String,
    },
    Move {
        id: String,
        after: Option<String>,
    },
}
```

Ràng buộc:

- `T` phải có duy nhất một `#[sync(id)]`
- mọi nested path được route qua identity đó
- index vật lý chỉ là view materialized, không phải identity sync

### 5.4 SyncableCounter

Dùng cho các giá trị đếm, revision, score, unread count.

```rust
pub enum CounterOp {
    Increment(i64),
    Decrement(i64),
}
```

Semantics v1:

- ưu tiên `increment/decrement`
- chưa đưa `reset` vào contract v1 để tránh ambiguity merge
- nếu cần reset, xử lý bằng state epoch ở tầng business hoặc bổ sung sau khi có
  semantics rõ hơn

### 5.5 SyncableMap<K, V>

Dùng cho map theo key logic. V1 ưu tiên `String` key.

```rust
pub enum MapOp {
    Insert {
        key: String,
        value: SnapshotValue,
    },
    Remove {
        key: String,
    },
    Replace {
        key: String,
        value: SnapshotValue,
    },
}
```

Map có thể chứa scalar syncable values hoặc nested syncable state.

Ví dụ path:

```text
settings[key="theme"]
presence[key="user_1"].cursor
```

`SnapshotValue` không phải blob vô kiểu. Nó là schema-backed snapshot fragment
của child node/container, có thể được encode trên wire bằng serde hoặc binary
codec, nhưng ở mức semantics vẫn là payload đã được ràng buộc bởi schema của
field đó. Khi apply `ListOp::Insert` hoặc `MapOp::{Insert, Replace}`, runtime
phải validate payload này khớp với schema của field đích. Nếu schema không khớp
hoặc version không tương thích, phải fail bằng lỗi apply rõ ràng thay vì cố
materialize một cách mơ hồ.

## 6. Derive macro contract

`#[derive(SyncableState)]` có trách nhiệm sinh schema và typed routing, không có
trách nhiệm intercept mọi phép gán field thường trong Rust.

### 6.1 Derive sẽ sinh

- `StateSchema` cho struct
- metadata field name và wire name
- routing `path -> field/container`
- validation compile-time cho các ràng buộc có thể kiểm tra sớm

### 6.2 Derive không làm

- không tự động biến mọi field thường thành syncable field
- không can thiệp trực tiếp vào assignment kiểu `doc.title = ...`
- không tạo business API magic khó debug

### 6.3 Attributes v1

- `#[sync(id)]`: đánh dấu field identity logic
- `#[sync(rename = "...")]`: đổi tên trong schema/wire path
- `#[sync(skip)]`: bỏ qua field local-only
- `#[sync(with = ...)]`: reserved cho adapter/custom codec; v1 parse nhưng compile-fail rõ ràng vì chưa hỗ trợ runtime behavior tương ứng

### 6.4 Cách viết derive ở mức crate

Nên tách thành ít nhất 2 crate:

- `syncable-state`: crate runtime, chứa trait, container, context, path, delta,
  snapshot, error types
- `syncable-state-derive`: crate `proc-macro`, chỉ chứa phần parse AST và sinh mã

Lý do:

- proc-macro bắt buộc phải ở crate riêng
- runtime không bị kéo theo phụ thuộc nặng như `syn`, `quote`, `proc-macro2`
- vòng đời bảo trì rõ ràng hơn: macro sinh code, runtime thực thi code đó

### 6.5 Input mà derive chấp nhận

V1 chỉ nên hỗ trợ:

- `struct` có named fields
- không hỗ trợ tuple struct
- không hỗ trợ enum làm root `SyncableState`
- generic chỉ hỗ trợ nếu mọi type parameter đều thỏa trait bound cần thiết

Ví dụ đầu vào hợp lệ:

```rust
#[derive(SyncableState)]
struct DocumentState {
    #[sync(id)]
    id: String,
    title: SyncableString,
    content: SyncableText,
}
```

Ví dụ đầu vào chưa hỗ trợ trong v1:

```rust
#[derive(SyncableState)]
struct BadState(SyncableString);
```

### 6.6 Những gì derive cần parse

Proc macro cần parse và chuẩn hóa các thông tin sau từ AST:

- tên struct
- danh sách field
- tên Rust của field
- tên wire/schema sau khi áp dụng `#[sync(rename = ...)]`
- field nào bị `#[sync(skip)]`
- field nào là `#[sync(id)]`
- field nào là syncable container hoặc nested syncable state

Đầu ra trung gian nên là một model nội bộ kiểu:

```rust
struct ParsedState {
    ident: syn::Ident,
    fields: Vec<ParsedField>,
    id_field: Option<usize>,
}

struct ParsedField {
    ident: syn::Ident,
    rust_ty: syn::Type,
    wire_name: String,
    is_id: bool,
    is_skipped: bool,
}
```

Nên có bước validate độc lập sau parse thay vì vừa parse vừa sinh mã. Điều này sẽ
giúp thông báo lỗi rõ hơn và giảm logic lồng nhau trong macro.

### 6.7 Những gì derive sẽ sinh ra

Với mỗi `#[derive(SyncableState)]`, macro nên sinh tối thiểu các phần sau:

- implementation của trait `SyncableState`
- một `Schema` tĩnh cho struct đó
- một `Snapshot` tương ứng nếu runtime tách riêng snapshot type
- logic route `SyncPath -> field`
- logic validate field metadata lúc compile-time ở mức có thể

Ví dụ ý tưởng đầu ra:

```rust
impl SyncableState for DocumentState {
    type Snapshot = DocumentStateSnapshot;
    type Schema = DocumentStateSchema;

    fn schema() -> &'static Self::Schema {
        &DOCUMENT_STATE_SCHEMA
    }

    fn snapshot(&self) -> Self::Snapshot {
        DocumentStateSnapshot {
            id: self.id.clone(),
            title: self.title.snapshot(),
            content: self.content.snapshot(),
        }
    }

    fn apply_at_path(
        &mut self,
        path: &[PathSegment],
        op: ChangeOp,
    ) -> Result<(), ApplyError> {
        // macro-generated field routing
    }
}
```

Macro không nên sinh business mutation methods như `rename_doc()` hay
`delete_doc()`. Những hàm đó thuộc app/domain layer.

### 6.8 Snapshot type do derive sinh

Để tránh lẫn lộn giữa runtime state và dữ liệu snapshot, derive nên sinh một kiểu
snapshot riêng cho mỗi state.

Ví dụ:

```rust
pub struct DocumentStateSnapshot {
    pub id: String,
    pub title: String,
    pub content: String,
}
```

Nguyên tắc chuyển đổi:

- `SyncableString` snapshot ra `String`
- `SyncableText` snapshot ra `String` hoặc text snapshot type riêng nếu cần
- `SyncableCounter` snapshot ra `i64`
- `SyncableVec<T>` snapshot ra `Vec<T::Snapshot>`
- `SyncableMap<String, V>` snapshot ra `BTreeMap<String, V::Snapshot>` hoặc map
  ổn định tương đương cho wire/output

Nhờ vậy API rõ hơn:

- runtime state tối ưu cho mutate/merge
- snapshot type tối ưu cho serialize, bootstrap client, test, và record

### 6.9 Compile-time validation mà derive cần làm

Macro nên fail sớm với các lỗi sau:

- có hơn một field `#[sync(id)]`
- `SyncableVec<T>` nhưng `T` không thỏa stable-id contract của thư viện; trong luồng derive bình thường điều này thường đến từ việc `T` không khai báo đúng một `#[sync(id)]`
- field `#[sync(id)]` lại bị `#[sync(skip)]`
- field được derive route đến nhưng type không implement trait sync tương ứng
- struct không có named fields

Thông báo lỗi cần trỏ đúng field và diễn đạt theo ngôn ngữ người dùng thư viện,
ví dụ:

```text
error: `SyncableVec<DocumentState>` requires `DocumentState` to declare exactly one `#[sync(id)]` field
```

### 6.10 Routing code mà derive nên sinh

Phần quan trọng nhất của derive là sinh field router đủ typed và đủ rõ để apply
remote delta.

Ví dụ ý tưởng sinh mã:

```rust
match path.first() {
    Some(PathSegment::Field("title")) => self.title.apply_path_tail(&path[1..], op),
    Some(PathSegment::Field("content")) => self.content.apply_path_tail(&path[1..], op),
    Some(PathSegment::Field("metadata")) => self.metadata.apply_path_tail(&path[1..], op),
    Some(PathSegment::Field(name)) => Err(ApplyError::UnknownField(name.to_string())),
    _ => Err(ApplyError::PathNotFound),
}
```

Với nested state hoặc nested container, derive chỉ route đến đúng child. Child sẽ
tự chịu trách nhiệm parse phần `path` còn lại.

### 6.11 Cấu trúc module gợi ý cho crate derive

```text
syncable-state-derive/
  src/
    lib.rs
    attrs.rs
    parse.rs
    validate.rs
    expand.rs
    snapshot.rs
    schema.rs
    diagnostics.rs
```

Ý nghĩa:

- `attrs.rs`: parse `#[sync(...)]`
- `parse.rs`: parse struct và field từ `syn`
- `validate.rs`: kiểm tra invariant trước khi expand
- `expand.rs`: sinh `impl SyncableState`
- `snapshot.rs`: sinh snapshot types và conversion
- `schema.rs`: sinh schema metadata tĩnh
- `diagnostics.rs`: chuẩn hóa compile errors

### 6.12 Thư viện hỗ trợ nên dùng

V1 nên dùng các crate quen thuộc của hệ sinh thái proc-macro:

- `syn` để parse AST
- `quote` để sinh token
- `proc-macro2` để thao tác token stream

Nếu attribute parsing bắt đầu nhiều nhánh hơn, có thể cân nhắc `darling`, nhưng
không bắt buộc cho v1. Ưu tiên của v1 là macro nhỏ, dễ đọc, và dễ debug.

### 6.13 Nguyên tắc thiết kế derive

Derive phải được xem là phần sinh glue code, không phải nơi giấu business logic.

Những gì nên nằm trong derive:

- schema metadata
- route path
- snapshot conversion
- trait impls lặp lại nhiều giữa các state

Những gì không nên nằm trong derive:

- logic merge chuyên biệt của từng container
- domain invariants nghiệp vụ
- transport decisions
- mutation orchestration của app

### 6.14 Pseudo-code cho `proc_macro_derive`

Luồng xử lý đề xuất cho macro:

```rust
#[proc_macro_derive(SyncableState, attributes(sync))]
pub fn derive_syncable_state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let parsed = match parse::parse_state(input) {
        Ok(v) => v,
        Err(err) => return err.into_compile_error().into(),
    };

    if let Err(err) = validate::validate_state(&parsed) {
        return err.into_compile_error().into();
    }

    let schema_tokens = schema::expand_schema(&parsed);
    let snapshot_tokens = snapshot::expand_snapshot(&parsed);
    let impl_tokens = expand::expand_syncable_state_impl(&parsed);

    quote! {
        #snapshot_tokens
        #schema_tokens
        #impl_tokens
    }
    .into()
}
```

Ý nghĩa của từng bước:

- `parse_state`: biến `DeriveInput` thành model nội bộ dễ thao tác
- `validate_state`: kiểm tra invariant và trả compile error sớm
- `expand_schema`: sinh metadata tĩnh cho field/router
- `expand_snapshot`: sinh snapshot type và conversion
- `expand_syncable_state_impl`: sinh `impl SyncableState`

Nguyên tắc quan trọng: mọi nhánh lỗi nên kết thúc bằng `compile_error!` có vị trí
trỏ tới field hoặc struct gây lỗi, không panic trong proc macro.

### 6.15 Skeleton gần thật cho crate `syncable-state-derive`

Đây là skeleton gợi ý đủ gần để hiện thực:

```rust
// src/lib.rs
mod attrs;
mod diagnostics;
mod expand;
mod parse;
mod schema;
mod snapshot;
mod validate;

use proc_macro::TokenStream;

#[proc_macro_derive(SyncableState, attributes(sync))]
pub fn derive_syncable_state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match parse::parse_state(input)
        .and_then(|parsed| validate::validate_state(&parsed).map(|_| parsed))
        .map(|parsed| {
            let schema_tokens = schema::expand_schema(&parsed);
            let snapshot_tokens = snapshot::expand_snapshot(&parsed);
            let impl_tokens = expand::expand_syncable_state_impl(&parsed);

            quote::quote! {
                #snapshot_tokens
                #schema_tokens
                #impl_tokens
            }
        }) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
```

```rust
// src/attrs.rs
pub struct SyncAttrs {
    pub is_id: bool,
    pub is_skip: bool,
    pub rename: Option<String>,
    pub with: Option<syn::Path>,
}

pub fn parse_sync_attrs(attrs: &[syn::Attribute]) -> syn::Result<SyncAttrs> {
    // parse #[sync(id)], #[sync(skip)], #[sync(rename = "...")], #[sync(with = path)]
}
```

```rust
// src/parse.rs
pub struct ParsedState {
    pub ident: syn::Ident,
    pub vis: syn::Visibility,
    pub generics: syn::Generics,
    pub fields: Vec<ParsedField>,
}

pub struct ParsedField {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub wire_name: String,
    pub attrs: crate::attrs::SyncAttrs,
}

pub fn parse_state(input: syn::DeriveInput) -> syn::Result<ParsedState> {
    // reject tuple structs and enums
    // collect named fields
    // parse attributes
}
```

```rust
// src/validate.rs
pub fn validate_state(parsed: &ParsedState) -> syn::Result<()> {
    // exactly one #[sync(id)] if needed by usage
    // no #[sync(id)] + #[sync(skip)]
    // field names unique after rename
    // optional trait-bound validation hints
    Ok(())
}
```

```rust
// src/snapshot.rs
pub fn expand_snapshot(parsed: &ParsedState) -> proc_macro2::TokenStream {
    // generate DocumentStateSnapshot
    // generate conversion from runtime fields to snapshot fields
}
```

```rust
// src/schema.rs
pub fn expand_schema(parsed: &ParsedState) -> proc_macro2::TokenStream {
    // generate field metadata and static schema value
}
```

```rust
// src/expand.rs
pub fn expand_syncable_state_impl(parsed: &ParsedState) -> proc_macro2::TokenStream {
    // generate impl SyncableState
    // generate apply_at_path routing
    // generate snapshot() delegation
}
```

```rust
// src/diagnostics.rs
pub fn error_spanned<T: quote::ToTokens>(tokens: T, message: &str) -> syn::Error {
    syn::Error::new_spanned(tokens, message)
}
```

Nếu muốn đi nhanh, có thể viết test cho từng module bằng snapshot testing trên
token output, nhưng v1 chỉ cần ưu tiên test parse/validate/expand ở các case chính.

### 6.16 Ví dụ expand gần thật cho `DocumentState`

Đầu vào:

```rust
#[derive(SyncableState)]
struct DocumentState {
    #[sync(id)]
    id: String,
    title: SyncableString,
    content: SyncableText,
}
```

Macro có thể expand ra gần giống như sau:

```rust
pub struct DocumentStateSnapshot {
    pub id: String,
    pub title: String,
    pub content: String,
}

pub struct DocumentStateSchema {
    pub fields: &'static [FieldSchema],
    pub id_field: &'static str,
}

static DOCUMENT_STATE_FIELDS: &[FieldSchema] = &[
    FieldSchema::new("id", FieldKind::Identity),
    FieldSchema::new("title", FieldKind::String),
    FieldSchema::new("content", FieldKind::Text),
];

static DOCUMENT_STATE_SCHEMA: DocumentStateSchema = DocumentStateSchema {
    fields: DOCUMENT_STATE_FIELDS,
    id_field: "id",
};

impl SyncableState for DocumentState {
    type Snapshot = DocumentStateSnapshot;
    type Schema = DocumentStateSchema;

    fn schema() -> &'static Self::Schema {
        &DOCUMENT_STATE_SCHEMA
    }

    fn snapshot(&self) -> Self::Snapshot {
        DocumentStateSnapshot {
            id: self.id.clone(),
            title: self.title.snapshot(),
            content: self.content.snapshot(),
        }
    }

    fn apply_at_path(
        &mut self,
        path: &[PathSegment],
        op: ChangeOp,
    ) -> Result<(), ApplyError> {
        match path.first() {
            Some(PathSegment::Field("title")) => self.title.apply_path_tail(&path[1..], op),
            Some(PathSegment::Field("content")) => self.content.apply_path_tail(&path[1..], op),
            Some(PathSegment::Field("id")) => Err(ApplyError::InvalidIdentityMutation),
            Some(PathSegment::Field(name)) => Err(ApplyError::UnknownField(name.to_string())),
            _ => Err(ApplyError::PathNotFound),
        }
    }

    fn identity(&self) -> Option<&str> {
        Some(self.id.as_str())
    }
}
```

Ví dụ này không phải token output cuối cùng phải giống 100%, nhưng nó giúp chốt
rõ 3 điều:

- snapshot type được sinh tự động
- schema metadata được sinh tự động
- field router được sinh tự động

### 6.17 Testing strategy cho derive crate

Derive crate nên có test ở 3 mức:

- parse tests: đảm bảo attribute được đọc đúng
- compile-fail tests: đảm bảo invariant sai báo lỗi đúng
- expansion behavior tests: đảm bảo type snapshot và routing được sinh đúng

Gợi ý:

- dùng `trybuild` cho compile-fail tests
- dùng unit test thường cho parse/validate
- chỉ so token string khi thực sự cần; ưu tiên test hành vi hơn test formatting

Các case tối thiểu nên có:

- derive thành công với state đơn giản
- fail khi có 2 field `#[sync(id)]`
- fail khi tuple struct derive `SyncableState`
- fail khi rename tạo trùng wire name
- thành công với nested `SyncableVec<T>` và sinh snapshot đúng kiểu

## 7. Runtime architecture

Kiến trúc runtime nên tách 3 lớp:

- `typed facade`: API mà app runtime và room logic gọi vào
- `schema/router`: path resolution, dispatch, validation
- `crdt core`: lưu internal metadata, merge semantics, và materialized state

Lợi ích:

- app code đơn giản và typed
- derive layer gọn và dễ suy luận
- merge engine và materialized state có thể test độc lập

Runtime public API nên tách local và remote rõ ràng:

```rust
pub trait SyncRuntime {
    type Snapshot;

    fn snapshot(&self) -> SnapshotBundle<Self::Snapshot>;
    fn poll_delta(&mut self) -> Option<DeltaBatch>;
    fn apply_remote(&mut self, batch: DeltaBatch) -> Result<(), ApplyError>;
}
```

Semantics:

- `poll_delta()` chỉ trả local committed batch chưa được tầng trên lấy đi
- `apply_remote()` chỉ apply batch đã được cấp `seq` bởi authoritative stream
- `apply_remote()` không tái phát batch đó như local delta một lần nữa
- `apply_remote()` chỉ advance local materialized state và sequence tracking
- nếu `batch.from_seq < local_seq`, v1 chỉ bỏ qua theo kiểu idempotent khi đó là
  replay thật sự của cùng authoritative stream; replay stale nhưng sai authority
  hoặc xung đột payload phải trả lỗi rõ ràng

## 8. Đồng bộ snapshot và delta

### 8.1 Contract tối thiểu của state

Mỗi state syncable phải cung cấp 2 năng lực cơ bản:

- chụp `snapshot` tại một `seq` hợp lệ
- phát `delta batch` cho mỗi mutation batch đã commit

Contract gợi ý:

```rust
pub trait SyncRuntime {
    type Snapshot;

    fn snapshot(&self) -> SnapshotBundle<Self::Snapshot>;
    fn poll_delta(&mut self) -> Option<DeltaBatch>;
}
```

### 8.2 Vai trò của seq

`seq` được đặt tại state/runtime level, sử dụng `ctx` để cấp phát.

`seq` giải các bài toán:

- snapshot alignment khi subscribe
- gap detection ở client
- replay theo thứ tự ở transport layer
- recovery khi reconnect

`seq` không tự nó giải bài toán true multi-writer merge. Nó là version sequence
của state stream, không phải toàn bộ metadata cho replication phức tạp.

### 8.3 Subscriber flow tối giản

Flow mong muốn:

1. room runtime verify subscribe
2. room runtime trả `snapshot(seq = N)`
3. từ thời điểm đó, subscriber handler nhận mọi `delta` có continuity bắt đầu
   bằng `from_seq == N`
4. handler lưu upstream `state seq` để replay, recover cho client

Nếu room/runtime đảm bảo không bị mất `delta` sau mốc subscribe, thì subscriber
handler có thể tự lưu và khôi phục event stream.

### 8.4 Điều kiện đúng của mô hình đơn giản này

Mô hình này chỉ đúng nếu `subscribe + snapshot + future deltas` được xử lý như
một contract tuyến tính trong cùng một execution boundary của room/runtime.

Nói cách khác:

- subscriber phải được gắn vào luồng nhận delta đúng mốc
- snapshot phải đại diện cho state tại một `seq` xác định
- mọi delta sau đó phải không bỏ sót đối với subscriber đã đăng ký

Topology được support trong v1 là: snapshot capture và subscriber registration
phải xảy ra atomically trong cùng room executor hoặc execution boundary đang sở
hữu authoritative stream.

Nếu snapshot và register subscriber bị tách rời ở hai boundary bất đồng bộ khác
nhau, vẫn có nguy cơ hụt event giữa snapshot và event đầu tiên.

## 9. Subscriber handler và transport boundary

Thư viện này chọn ranh giới đơn giản:

- `syncable-state` chỉ cấp `snapshot + delta + seq`
- tầng subscriber handler (websocket, TCP, ...) tự lo:
  - lưu outbound event log theo upstream `state seq`
  - replay theo `seq`
  - gap detection và catch-up
  - framing và retry

Lợi ích:

- core library giữ được logic rất gọn
- transport không bị khóa chặt vào CRDT internals
- websocket handler có thể tự quyết định chính sách lưu event và khôi phục

Rủi ro cần ghi rõ:

- nếu log chỉ tồn tại trong memory của một handler, crash có thể làm mất khả
  năng replay
- nếu reconnect sang node/handler khác, log cũ có thể không còn
- nếu event log chỉ tồn tại cục bộ theo handler, reconnect sang handler khác có
  thể không replay đủ được

V1 chấp nhận các hạn chế trên để đổi lại kiến trúc rất đơn giản. Nếu cần support
multi-node replay bền và reconnect linh hoạt, cần thêm shared stream log ở tầng
room/channel trong phase sau.

## 10. Replication model

### 10.1 V1: single authoritative writer

Để giữ thiết kế dễ hiểu và dễ vận hành, v1 giả định mỗi state/room có một
authoritative writer tại một thời điểm.

Hệ quả kiến trúc:

- `seq` tăng đơn điệu và dễ suy luận
- snapshot và delta có thứ tự rõ ràng
- subscriber catch-up đơn giản
- replication ra follower hoặc transport không cần true multi-writer conflict
  resolution đầy đủ ngay lập tức

Remote apply trong v1 được hiểu là: follower hoặc replica phụ nhận `DeltaBatch`
đã được sequencing bởi authoritative writer, validate continuity `from_seq ==
local_seq`, apply vào state, rồi cập nhật local `seq = to_seq` mà không re-emit
batch đó như local change mới.

### 10.2 Đường mở rộng: multi-writer

Nếu sau này cần true multi-writer replication, cần bổ sung metadata như:

- `replica_id`
- `op_id`
- `version vector` hoặc `deps`

Nhưng mô hình API typed, path, ctx, và container contract vẫn có thể giữ nguyên.

### 10.3 Đề xuất cơ chế `seq` cho multi-writer

Khi bước sang multi-writer thật sự, không nên cố duy trì một `seq` toàn cục duy
nhất ngay ở core state. Đó không phải cách phần lớn thư viện CRDT xử lý.

Các thư viện CRDT phổ biến thường dùng mô hình gần giống nhau:

- mỗi replica có một định danh riêng
- mỗi replica tự tăng clock/counter cục bộ cho các thay đổi nó tạo ra
- trạng thái đồng bộ giữa các replica được mô tả bằng `version vector`,
  `state vector`, `frontier`, hoặc `heads`
- nếu cần delivery order cho transport/subscriber, lớp stream phía trên mới gán
  thêm một `stream_seq`

Nói cách khác:

- `global seq` phù hợp cho authoritative stream hoặc broadcast stream
- `per-replica seq` mới là thứ phù hợp cho true multi-writer CRDT core

### 10.4 Cách các thư viện CRDT phổ biến đang làm

#### Yjs

Yjs dùng `client id + clock` và một `state vector` để mô tả replica đang có đến
đâu. Khi sync, một bên gửi state vector của mình, bên kia tính phần thiếu và gửi
đúng các update còn thiếu.

Điểm rút ra:

- không cần `seq` toàn cục cho merge
- chỉ cần biết mỗi replica đã tạo bao nhiêu thay đổi
- diff được tính từ `state vector`

#### Automerge

Automerge gắn identity cho các operation/change và dùng `heads` hoặc lịch sử
thay đổi để biết hai phía đang lệch nhau ở đâu. Merge diễn ra dựa trên causal
history chứ không dựa trên một số thứ tự toàn cục duy nhất.

Điểm rút ra:

- merge dựa trên causal graph
- list/text dùng identity của element và reference tới operation trước đó
- transport có thể gửi incremental changes mà không cần global seq ở core

#### Loro

Loro tách khá rõ `peer id`, `oplog/docstate`, `frontiers`, và `version vector`.
Đây là một mô hình rất gần với hướng nên dùng cho thư viện của chúng ta nếu muốn
multi-writer về sau.

Điểm rút ra:

- lưu event/op log riêng với materialized state là rất có ích
- `frontier` là neo tốt cho snapshot, checkout, replay, và sync diff
- `peer id + counter` là nền tảng tốt hơn một global seq ở core

### 10.5 Đề xuất kiến trúc version cho thư viện này

Nên tách rõ 3 lớp version/order khác nhau:

- `LocalCounter`: số tăng đơn điệu của một replica
- `VersionVector` hoặc `Frontier`: mô tả replica hiện đang biết đến đâu trên toàn
  mạng
- `StreamSeq`: số thứ tự delivery ở tầng subscriber/transport, nếu cần

Mô hình dữ liệu gợi ý:

```rust
pub struct ReplicaId(String);

pub struct OpId {
    pub replica_id: ReplicaId,
    pub counter: u64,
}

pub struct VersionVector {
    pub clocks: BTreeMap<ReplicaId, u64>,
}

pub struct Frontier {
    pub heads: Vec<OpId>,
}
```

Trong đó:

- `OpId` định danh duy nhất cho một operation hoặc một committed batch cục bộ
- `VersionVector` thuận tiện cho diff và handshake sync
- `Frontier` thuận tiện cho snapshot, causal checkpoint, và oplog traversal

### 10.6 Nên gắn seq vào mức nào

Đề xuất thực tế nhất là:

- v1 hiện tại giữ `seq: u64` như state stream sequence vì đang giả định single
  authoritative writer
- khi mở sang multi-writer, đổi nghĩa phần core từ `seq` sang `OpId + Frontier`
- chỉ giữ `stream_seq` ở tầng transport nếu vẫn cần replay cho client

Nói gọn:

- core merge: dùng `OpId`, `VersionVector`, `Frontier`
- subscriber replay: dùng `stream_seq`

Không nên cố ép một con số `seq: u64` duy nhất giải quyết đồng thời cả merge,
causality, replay, replication, và broadcast ordering.

### 10.7 Đơn vị đánh số nên là operation hay batch

Với thư viện này, nên đánh số theo `batch` thay vì từng primitive operation.

Ví dụ:

- user đổi title
- đồng thời tăng revision counter
- toàn bộ hai thay đổi này được commit thành một `ChangeBatch`

Khi đó mỗi batch có một `BatchId`:

```rust
pub struct BatchId {
    pub replica_id: ReplicaId,
    pub counter: u64,
}

pub struct ChangeBatch {
    pub id: BatchId,
    pub base: Frontier,
    pub changes: Vec<ChangeEnvelope>,
}
```

Ưu điểm:

- khớp với `BatchTx` đã có trong thiết kế
- dễ record và replay hơn
- sát mental model domain operation hơn
- transport gọn hơn rất nhiều

### 10.8 Snapshot trong multi-writer nên mang gì

Nếu thư viện đi theo multi-writer, snapshot không nên chỉ mang `seq: u64` nữa.
Nó nên mang một causal checkpoint.

Ví dụ:

```rust
pub struct SnapshotBundle<TSnapshot> {
    pub frontier: Frontier,
    pub version: VersionVector,
    pub snapshot: TSnapshot,
}
```

Vai trò:

- `frontier` cho biết snapshot này tương ứng với causal heads nào
- `version` cho biết đã nhìn thấy thay đổi từ từng replica đến đâu
- peer khác có thể dùng thông tin này để tính diff còn thiếu

### 10.9 Giao thức sync giữa hai replica

Luồng sync khuyến nghị cho multi-writer:

1. replica A gửi `VersionVector` hoặc `Frontier` hiện tại cho replica B
2. replica B tính các `ChangeBatch` mà A còn thiếu
3. B gửi các batch thiếu về cho A
4. A import các batch này theo causal order nếu có thể
5. nếu có batch chưa đủ dependency, đưa vào pending log đợi batch còn thiếu

Điều này rất gần với cách Yjs dùng state vector và cách Loro/Automerge suy luận
trên causal history.

### 10.10 Pending ops và import order

Khi multi-writer, không thể giả định batch luôn đến theo đúng thứ tự causal.
Vì vậy runtime nên chuẩn bị một lớp pending/import queue:

- nếu dependencies đã đủ, apply ngay
- nếu thiếu dependency, giữ batch ở pending store
- khi dependency đến, thử apply lại

Ví dụ dữ liệu:

```rust
pub struct PendingImportStore {
    by_missing_dep: BTreeMap<BatchId, Vec<ChangeBatch>>,
}
```

Như vậy merge core sẽ bền hơn trước out-of-order delivery hoặc sync từng phần.

### 10.11 Khuyến nghị cụ thể cho thư viện này

Nếu muốn đi đến multi-writer mà không phá thiết kế hiện tại, lộ trình hợp lý là:

#### Phase A

- giữ `seq: u64` cho v1 single-authoritative writer
- hoàn thiện typed API, batching, snapshot, transport replay

#### Phase B

- đổi internal batch identity thành `BatchId { replica_id, counter }`
- thêm `Frontier` và `VersionVector` vào snapshot và replication protocol
- tách `stream_seq` của subscriber ra khỏi version của CRDT core

#### Phase C

- thêm pending import queue
- thêm diff protocol dựa trên `VersionVector`
- cho phép nhiều writer tạo batch đồng thời

### 10.12 Kết luận thiết kế

Đề xuất mạnh nhất là:

- đừng dùng một `seq` toàn cục duy nhất cho multi-writer core
- hãy dùng `replica_id + local counter` để định danh batch
- dùng `VersionVector` hoặc `Frontier` để biểu diễn causal progress
- nếu cần replay cho client, dùng thêm `stream_seq` ở tầng delivery

Đây là hướng gần với cách các thư viện CRDT trưởng thành đang làm, đồng thời vẫn
khớp với kiến trúc hiện tại của tài liệu này:

- typed API không đổi
- `BatchTx` vẫn đúng
- `ChangeEnvelope` vẫn đúng
- transport layer vẫn có thể replay đơn giản
- replication layer mới là nơi mang causal metadata mạnh hơn

### 10.13 Next step thực thi cho multi-writer với `VersionVector`

Nếu muốn bắt đầu chuẩn bị multi-writer mà chưa phá vỡ v1, thứ tự làm nên là như
sau:

#### Bước 1: Giữ nguyên API v1, bổ sung metadata nội bộ

Chưa thay public API `snapshot(seq)` và `DeltaBatch { from_seq, to_seq }` ngay.
Trước hết chỉ thêm nội bộ:

- `ReplicaId`
- `BatchId { replica_id, counter }`
- `VersionVector`
- `Frontier`

Mục tiêu là cho runtime bắt đầu lưu causal metadata mà chưa làm gãy subscriber
flow hiện tại.

#### Bước 2: Đổi `ChangeCtx` thành cấu trúc hai tầng

Từ:

```rust
pub struct ChangeCtx {
    replica_id: ReplicaId,
    next_seq: u64,
    open_batch: Vec<ChangeEnvelope>,
    pending: Vec<DeltaBatch>,
}
```

Tiến dần tới:

```rust
pub struct ChangeCtx {
    replica_id: ReplicaId,
    next_local_counter: u64,
    local_version: VersionVector,
    frontier: Frontier,
    open_batch: Vec<ChangeEnvelope>,
    pending: Vec<CommittedBatch>,
}

pub struct CommittedBatch {
    pub id: BatchId,
    pub base_frontier: Frontier,
    pub version_after: VersionVector,
    pub changes: Vec<ChangeEnvelope>,
}
```

Ý nghĩa:

- `next_local_counter` thay vai trò `seq` ở core multi-writer
- `base_frontier` cho biết batch được tạo dựa trên causal heads nào
- `version_after` cho biết sau khi commit, replica đã tiến tới đâu

#### Bước 3: Thêm handshake sync bằng `VersionVector`

API tối thiểu nên có:

```rust
pub trait MultiWriterSyncRuntime {
    type Snapshot;

    fn snapshot_with_version(&self) -> SnapshotBundle<Self::Snapshot>;
    fn version_vector(&self) -> VersionVector;
    fn export_since(&self, remote: &VersionVector) -> Vec<CommittedBatch>;
    fn import_batch(&mut self, batch: CommittedBatch) -> ImportResult;
}
```

Flow sync:

1. A gửi `VersionVector` hiện tại cho B
2. B gọi `export_since(&vector_cua_a)`
3. B gửi về các `CommittedBatch` A còn thiếu
4. A import từng batch
5. batch nào thiếu dependency thì để pending

Đây là bước quan trọng nhất để chuyển từ replay tuyến tính sang sync causal đúng
nghĩa.

#### Bước 4: Thêm pending import queue

Multi-writer gần như chắc chắn cần import out-of-order. Vì vậy nên thêm sớm:

```rust
pub struct PendingImportStore {
    by_missing_dep: BTreeMap<BatchId, Vec<CommittedBatch>>,
}
```

Rule:

- nếu `base_frontier` đã được thỏa, apply ngay
- nếu chưa, đưa batch vào pending
- mỗi lần import thành công, thử drain lại pending queue

#### Bước 5: Tách `stream_seq` khỏi version của core

Khi multi-writer đã có `VersionVector`, tầng subscriber phải ngừng dựa hoàn toàn
vào `seq` của core để suy luận replication.

Thay vào đó:

- core trả causal metadata: `BatchId`, `Frontier`, `VersionVector`
- subscriber handler tự gán `stream_seq` cho delivery order nếu cần
- reconnect client dùng `stream_seq` để bắt event log
- replica sync với nhau dùng `VersionVector`

Điều này giúp tách 2 concern đang khác nhau bản chất:

- `causal sync giữa replicas`
- `ordered delivery cho subscribers`

#### Bước 6: Nâng `SnapshotBundle`

Khi sẵn sàng chuyển phase, snapshot nên nâng từ:

```rust
pub struct SnapshotBundle<TSnapshot> {
    pub seq: u64,
    pub snapshot: TSnapshot,
}
```

thành:

```rust
pub struct SnapshotBundle<TSnapshot> {
    pub frontier: Frontier,
    pub version: VersionVector,
    pub snapshot: TSnapshot,
}
```

Nếu cần tương thích ngược trong giai đoạn chuyển tiếp, có thể giữ thêm:

```rust
pub struct SnapshotBundle<TSnapshot> {
    pub stream_seq: Option<u64>,
    pub frontier: Frontier,
    pub version: VersionVector,
    pub snapshot: TSnapshot,
}
```

#### Bước 7: Đổi roadmap implementation theo 2 nhánh rõ ràng

Nên chia roadmap sau này thành:

- `delivery track`: subscriber replay, websocket catch-up, stream log
- `replication track`: version vector, causal diff, pending import, frontier

Làm như vậy sẽ tránh nhầm lẫn giữa hai loại `seq`:

- `stream_seq` cho delivery
- `counter` trong `BatchId` cho causal replication

### 10.14 Khuyến nghị thực tế cho giai đoạn tiếp theo

Next step hợp lý nhất ngay sau tài liệu này là:

1. thêm `ReplicaId`, `BatchId`, `VersionVector`, `Frontier` vào model nhưng chưa
   public hóa toàn bộ
2. refactor `ChangeCtx` để batch commit có thể mang `base_frontier`
3. thêm `export_since(&VersionVector)` và `import_batch()` ở dạng trait nội bộ
4. giữ nguyên subscriber replay v1 bằng `seq`
5. chỉ khi replication track ổn định mới nâng `SnapshotBundle` sang causal version

Đây là cách ít rủi ro nhất vì:

- không phá flow đang đơn giản của v1
- không buộc transport phải đổi cùng lúc
- tạo được đường chuyển mượt sang multi-writer thật sự

## 11. Error handling

V1 cần có các nhóm lỗi rõ ràng:

- `ApplyError::PathNotFound`
- `ApplyError::OpKindMismatch`
- `ApplyError::UnknownField`
- `ApplyError::MissingIdentity`
- `MutationError::ItemNotFound`
- `MutationError::DuplicateIdentity`
- `MutationError::InvalidOperation`
- `SyncError::GapDetected`
- `SyncError::SeqContinuityViolation`

Nguyên tắc:

- typed route sai phải fail sớm
- op sai loại dữ liệu phải fail rõ ràng
- mutation local vi phạm invariant phải bị chặn trước khi emit delta

## 12. Testing strategy

### 12.1 Unit tests cho container

Mỗi container cần có test riêng cho:

- local mutation
- nested path generation
- apply remote delta
- idempotent apply nếu cần
- edge cases và invalid input

### 12.2 Property và invariant tests

Cần test các invariant sau:

- snapshot -> apply deltas -> hội tụ đúng state
- delete/move/insert trong `SyncableVec` không làm mất identity
- `seq` tăng đơn điệu, không nhảy lung tung trong local mutation flow
- client gap detection đúng khi thiếu batch
- `apply_remote()` không re-emit local delta
- replay continuity fail sớm nếu `from_seq != local_seq`

### 12.3 Integration tests

Cần có integration tests ở mức room/runtime:

- subscriber mới nhận snapshot đúng seq
- subscriber nhận đủ mọi delta sau mốc subscribe
- handler replay được từ seq cũ
- fallback sang full snapshot khi không replay đủ event log

## 13. Roadmap hiện thực hóa

### Phase 1: core scaffolding

- tạo crate `syncable-state`
- định nghĩa `ChangeCtx`, `BatchTx`, `SnapshotBundle`, `DeltaBatch`, `SyncPath`,
  `ChangeOp`
- hiện thực `SyncableString`, `SyncableCounter`
- tạo derive macro `SyncableState` ở mức tối thiểu

### Phase 2: nested structures

- thêm `SyncableVec<T>` với `#[sync(id)]`
- thêm `SyncableMap<String, V>`
- hoàn thiện nested path routing và remote apply

### Phase 3: text và subscriber flow

- thêm `SyncableText`
- bổ sung API `snapshot()` và `poll_delta()` đầy đủ
- tích hợp thử nghiệm với room document app

### Phase 4: transport integration

- kết nối với subscriber handler hiện có
- lưu outbound delta log theo `seq`
- thêm replay/catch-up flow cho websocket clients

### Phase 5: replication evolution

- đánh giá nhu cầu multi-writer thật sự
- nếu cần, bổ sung `op_id/deps/replica frontier`
- giữ nguyên typed API và transport boundary đã thiết kế

## 14. Quyết định chốt cho v1

- dùng `custom CRDT core`, không phụ thuộc backend có sẵn
- typed API là ưu tiên số 1
- `ChangeOp` là enum bọc các op con theo từng loại dữ liệu
- collection path dựa trên stable identity, không dựa trên index vật lý
- `ctx` sở hữu `replica_id` và `seq`
- chỉ authoritative stream được cấp `seq`
- snapshot luôn đi kèm `seq`
- delta luôn đi theo `from_seq -> to_seq`
- subscriber handler tự lo lưu log, replay, và recover
- replication v1 giả định `single authoritative writer`

## 15. Ví dụ sử dụng mong muốn

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
    metadata: SyncableMap<String, SyncableString>,
}

impl DocumentApp {
    fn delete_doc(&mut self, doc_id: &str) -> Result<(), DocumentError> {
        let mut batch = self.ctx.begin_batch()?;
        self.state.docs.delete(&mut batch, doc_id)?;
        let _ = batch.commit();
        Ok(())
    }

    fn rename_doc(&mut self, doc_id: &str, title: String) -> Result<(), DocumentError> {
        let mut batch = self.ctx.begin_batch()?;
        let doc = self.state.docs.get_mut(doc_id)?;
        doc.title.set(&mut batch, title);
        doc.revision.increment(&mut batch, 1);
        let _ = batch.commit();
        Ok(())
    }

    fn subscribe_init(&self) -> SnapshotBundle<DocumentAppStateSnapshot> {
        self.state.snapshot()
    }

    fn poll_sync(&mut self) -> Option<DeltaBatch> {
        self.ctx.poll()
    }
}
```

Đây là hướng thiết kế ưu tiên sự rõ ràng, typed safety, và khả năng tiến hóa dần.
Nó cố ý giữ cho v1 đơn giản nhưng không đóng sập cửa để mở rộng cho replay,
recover, và replication nặng hơn trong các phase tiếp theo.
