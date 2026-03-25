//! A derivation macro for the `syncable-state` library.
//!
//! This crate provides the `#[derive(SyncableState)]` macro, which automatically
//! implements the `SyncableState` trait for structs. It generates the necessary
//! schema and synchronization path bindings for replicating the struct's fields
//! across the network.
//!
//! # Example
//!
//! ```rust
//! use syncable_state_derive::SyncableState;
//! use syncable_state::{SyncableState, SyncableString, SyncPath, PathSegment};
//!
//! #[derive(SyncableState, Clone)]
//! pub struct UserProfile {
//!     #[sync(id)]
//!     pub id: String,
//!     pub name: SyncableString,
//! }
//! ```

mod attrs;
mod diagnostics;
mod expand;
mod parse;
mod schema;
mod snapshot;
mod validate;

use proc_macro::TokenStream;

/// Derives the `SyncableState` trait for a struct.
///
/// Automatically generates the schema, path bindings, and remote component
/// logic for a structure, allowing it to be managed inside a `RuntimeState`
/// or embedded in other syncable containers.
///
/// # Attributes
///
/// - `#[sync(id)]`: Designates a field as the unique identifier for instances
///   of this struct. This is required when embedding the struct inside a
///   `SyncableMap`, as the map needs a way to extract the key from the element.
///
/// # Usage
///
/// Typically, you will also want to derive `Clone` for your state structs,
/// as the `RuntimeState` uses clone-based rollback semantics for failed transaction
/// batches.
///
/// ```rust
/// use syncable_state_derive::SyncableState;
/// use syncable_state::{SyncableState, SyncableCounter, SyncPath};
///
/// #[derive(SyncableState, Clone)]
/// pub struct PlayerStats {
///     #[sync(id)]
///     pub player_id: String,
///     pub score: SyncableCounter,
/// }
/// ```
#[proc_macro_derive(SyncableState, attributes(sync))]
pub fn derive_syncable_state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match expand::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => diagnostics::into_compile_error(error).into(),
    }
}
