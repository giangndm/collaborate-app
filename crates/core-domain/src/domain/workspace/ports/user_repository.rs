use crate::workspace::{User, UserId, WorkspaceId, WorkspaceResult};
use async_trait::async_trait;

/// Abstracts access to the global user records the workspace domain depends on.
///
/// Use this port when workspace use cases need typed user identity or account
/// state while keeping persistence and lookup strategy outside the domain.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Returns an error when the user is absent.
    async fn get(&self, user_id: &UserId) -> WorkspaceResult<User>;

    /// Returns users for the provided ids in storage-defined order.
    async fn list_by_ids(&self, user_ids: &[UserId]) -> WorkspaceResult<Vec<User>>;

    /// Returns users who can be invited to a workspace (excluding current members).
    async fn find_candidates(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> WorkspaceResult<Vec<User>>;

    /// Returns all users in the system.
    async fn list_all(&self) -> WorkspaceResult<Vec<User>>;

    /// Saves or updates a user record.
    async fn save(&self, user: &User) -> WorkspaceResult<()>;
}
