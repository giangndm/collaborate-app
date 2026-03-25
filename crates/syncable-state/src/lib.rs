//! The `syncable-state` library for deterministic, high-performance CRDT-like state synchronization.
//!
//! This crate provides a framework for defining hierarchical, syncable state models. It is built
//! around the concept of an **Authoritative Writer v1** model, where continuous sequences of state
//! changes originate from a single authority, while other peers apply these changes deterministically.
//!
//! # Core Concepts
//!
//! - **[`SyncableState`]**: A trait implemented by state objects to support synchronization. It provides
//!   hooks for applying deltas, capturing snapshots, and defining the data schema.
//!   Typically derived using `#[derive(SyncableState)]`.
//! - **[`RuntimeState`]**: The core execution engine wrapped around a `SyncableState`. It tracks
//!   transaction sequences, manages applied remote deltas, and accumulates local changes.
//! - **[`DeltaBatch`]**: A cohesive batch of changes (`ChangeEnvelope`s) mapped to a specific sequence
//!   number (`seq`). Batches are guaranteed atomic when applied remotely.
//!
//! # Getting Started
//!
//! ```rust
//! use syncable_state::{SyncableState, RuntimeState, SyncableCounter, SyncableString, SyncPath, SyncError};
//!
//! // 1. Define your syncable state schema using the derive macro
//! #[derive(SyncableState, Clone)]
//! pub struct Document {
//!     #[sync(id)]
//!     pub id: String,
//!     pub title: SyncableString,
//!     pub views: SyncableCounter,
//! }
//!
//! impl Document {
//!     pub fn new(id: impl Into<String>) -> Self {
//!         let id = id.into();
//!         Self {
//!             id: id.clone(),
//!             title: SyncableString::from(""),
//!             views: SyncableCounter::from(0),
//!         }
//!     }
//! }
//!
//! fn example() -> Result<(), SyncError> {
//!     // 2. Wrap state in a RuntimeState for the active peer
//!     let doc = Document::new("doc-1");
//!     let mut local_peer = RuntimeState::new("peer-A", doc.clone());
//!     let mut remote_peer = RuntimeState::new("peer-B", doc);
//!
//!     // 3. Perform a local batch transaction
//!     local_peer.with_batch(|state, batch| {
//!         state.title.set(batch, "Hello World")?;
//!         state.views.increment(batch, 1)?;
//!         Ok::<(), SyncError>(())
//!     })?;
//!
//!     // 4. Poll the generated delta and apply it remotely
//!     if let Some(delta) = local_peer.poll() {
//!         remote_peer.apply_remote(delta)?;
//!     }
//!     
//!     assert_eq!(local_peer.state().title.value(), remote_peer.state().title.value());
//!     Ok(())
//! }
//! ```

mod change;
pub mod containers;
mod context;
mod error;
mod path;
mod schema;
mod snapshot;
mod traits;

pub use change::{ChangeEnvelope, ChangeOp, CounterOp, ListOp, MapOp, StringOp, TextOp};
pub use containers::{SyncableCounter, SyncableMap, SyncableString, SyncableText, SyncableVec};
pub use context::{ChangeCtx, RuntimeState};
pub use error::SyncError;
pub use path::{PathSegment, ReplicaId, SyncPath};
pub use schema::{FieldKind, FieldSchema, StateSchema};
pub use snapshot::{BatchProof, DeltaBatch, RuntimeBootstrap, SnapshotBundle, SnapshotValue};
pub use traits::{
    ApplyChildPath, ApplyPath, CounterContainer, EventTracker, SnapshotCodec, StableId,
    StringContainer, SyncContainer, SyncRuntime, SyncableState, TextContainer,
};

#[cfg(feature = "derive")]
pub use syncable_state_derive::SyncableState;
