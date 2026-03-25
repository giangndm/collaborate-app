# Syncable-State Derive: Under the Hood

The `syncable-state-derive` crate provides the `#[derive(SyncableState)]` macro. This macro generates the necessary boilerplate to integrate user-defined Rust structs into the `syncable-state` synchronization engine.

This document explains what happens when you use `#[derive(SyncableState)]`, how it interacts with the runtime data models, and how it handles unique identity tracking for collections.

---

## 1. What Actually Happens When You `#[derive(SyncableState)]`?

When you annotate a struct with the derive macro:

```rust
#[derive(SyncableState)]
pub struct UserProfile {
    #[sync(id)]
    pub id: String,
    pub name: SyncableString,
}
```

The procedural macro parses your struct via `syn` and generates the following implementations behind the scenes:

### A. Implementing `SyncableState`

It automatically writes the `impl SyncableState for UserProfile` block. This implementation provides three core capabilities:

1. **`snapshot()`**: It generates code to capture the state of every field. It iterates over your fields and calls `.snapshot()` on them.
2. **`schema()`**: It generates a static `StateSchema` that describes the data types of your struct so that the runtime can validate incoming remote changes against your structural definition.
3. **`rebind_paths()`**: It updates the `SyncPath` of every nested `Syncable*` container inside the struct. If `UserProfile` is nested inside a `SyncableMap` under the key `"user-123"`, the `rebind_paths` method recursively passes `[MapKey("user-123"), Field("name")]` down to the `SyncableString`.

### B. Implementing `ApplyPath` & `ApplyChildPath`

Your struct must know how to route incoming remote patches to the correct child field. The macro generates a massive `match` statement.

When a `ChangeEnvelope` arrives with the path `[Field("name")]` and a string `ChangeOp`:

```rust
impl ApplyPath for UserProfile {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match path.first() {
            Some(PathSegment::Field("name")) => {
                // Route the op strictly to the `name` field
                self.name.apply_path_tail(&path[1..], op)
            }
            Some(PathSegment::Field("id")) => { ... }
            _ => Err(SyncError::InvalidPath),
        }
    }
}
```

This guarantees strict type safety. A patch meant for a `SyncableCounter` will never be accidentally routed to a `SyncableString` during traversal.

### C. Implementing `SnapshotCodec`

The macro generates standard conversion routines to translate your in-memory struct into a serialized `SnapshotValue` tree, and vice-versa.

When your struct is reconstructed from a network sync snapshot, the macro parses the raw tree and invokes `SnapshotCodec::snapshot_from_value` on each child primitive.

---

## 2. Tracking ID in Collections (Maps & Lists)

The most complex requirement of the derivation is tracking identity correctly across dynamically nested collections. It solves this using the `#[sync(id)]` attribute.

### The Role of `#[sync(id)]`

To insert an element into a `SyncableMap<V>` or a `SyncableVec<V>`, the element `V` **must** possess a stable identity so the CRDT conflict resolver knows how to address it.

If you declare a field with `#[sync(id)]`:

```rust
#[derive(SyncableState)]
pub struct DocumentNode {
    #[sync(id)] // Explicit marker
    pub node_id: String,
    pub content: SyncableText,
}
```

The macro recognizes `node_id` as the authoritative identity for `DocumentNode`.

### How `StableId` is Implemented

The macro automatically generates an `impl StableId for DocumentNode`:

```rust
impl StableId for DocumentNode {
    fn stable_id(&self) -> &str {
        &self.node_id
    }
}
```

### Map Insertion Flow

When you call `map.insert(batch, "doc-1", new_node)`:

1. The `SyncableMap` takes the string `"doc-1"` and your `new_node` struct.
2. The `SyncableMap` expects the identity of the inserted struct to perfectly align with the logical mapping.
3. Currently, `SyncableMap` handles the identity manually for its internal `BTreeMap`, but it enforces that nested structures correctly derive `StableId` to ensure the overall schema remains intact across dynamic network topologies.

### List Insertion Flow (`SyncableVec`)

If you were inserting into an ordered `SyncableVec`:

1. The `Vec` uses `StableId` to determine the unique key for the item.
2. When the `Vec` computes a patch like `ListOp::Insert { id: "doc-1", after: "doc-0" }`, it natively relies on the `StableId` lookup to compute logical CRDT interleaving points.
3. This prevents duplicate items or conflicting identities when multiple remote peers insert items concurrently without knowing the true physical index of the array!

### Identity Field Extraction Rules

- The macro demands that a `#[sync(id)]` field must be an owned `String` in v1.
- You **cannot** have more than one `#[sync(id)]` field per struct. The compiler macro will panic and give a clear syntax error.
- You **cannot** use `#[sync(skip)]` on the `#[sync(id)]` field. The identity _must_ be replicated across the network, otherwise remote peers cannot reference the container's children.
