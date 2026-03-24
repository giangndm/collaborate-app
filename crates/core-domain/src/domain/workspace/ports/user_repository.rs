use crate::workspace::{User, UserId, WorkspaceResult};

/// Abstracts access to the global user records the workspace domain depends on.
///
/// Use this port when workspace use cases need typed user identity or account
/// state while keeping persistence and lookup strategy outside the domain.
pub trait UserRepository {
    /// Returns an error when the user is absent.
    fn get(&self, user_id: &UserId) -> WorkspaceResult<User>;

    /// Returns users for the provided ids in storage-defined order.
    fn list_by_ids(&self, user_ids: &[UserId]) -> WorkspaceResult<Vec<User>>;
}
