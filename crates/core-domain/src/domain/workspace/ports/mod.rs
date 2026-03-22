//! Outbound dependency traits for the workspace domain live here.

mod membership_repository;
mod secret_store;
mod user_repository;
mod workspace_repository;

pub use membership_repository::*;
pub use secret_store::*;
pub use user_repository::*;
pub use workspace_repository::*;
