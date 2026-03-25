use crate::workspace::{
    UserId, WorkspaceId, WorkspaceMembership, WorkspaceMembershipId, WorkspaceResult,
};
use async_trait::async_trait;

/// Abstracts persistence for workspace membership assignments.
///
/// Use this port when a use case needs to answer who belongs to a workspace,
/// what role they hold there, or to persist membership changes without binding
/// the domain to a specific backing store.
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// Returns an error when the membership is absent.
    async fn get(
        &self,
        membership_id: &WorkspaceMembershipId,
    ) -> WorkspaceResult<WorkspaceMembership>;

    /// Returns an error when the membership is absent for that workspace-user pair.
    async fn find_for_workspace_user(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> WorkspaceResult<WorkspaceMembership>;

    /// Returns memberships for one user across workspaces in storage-defined order.
    async fn list_for_user(&self, user_id: &UserId) -> WorkspaceResult<Vec<WorkspaceMembership>>;

    /// Returns memberships in storage-defined order; callers must not rely on sorting.
    async fn list_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<Vec<WorkspaceMembership>>;

    /// Removes the membership when it exists.
    async fn remove(&self, membership_id: &WorkspaceMembershipId) -> WorkspaceResult<()>;

    /// Saves a new membership and bumps the owning workspace timestamp atomically.
    async fn save_with_workspace_bump(
        &self,
        membership: &WorkspaceMembership,
    ) -> WorkspaceResult<()>;

    /// Removes a membership and bumps the owning workspace timestamp atomically.
    async fn remove_with_workspace_bump(
        &self,
        workspace_id: &WorkspaceId,
        membership_id: &WorkspaceMembershipId,
    ) -> WorkspaceResult<()>;

    /// Saves the membership as an upsert.
    async fn save(&self, membership: &WorkspaceMembership) -> WorkspaceResult<()>;
}
