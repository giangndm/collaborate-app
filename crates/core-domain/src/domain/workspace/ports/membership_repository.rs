use crate::workspace::{
    UserId, WorkspaceId, WorkspaceMembership, WorkspaceMembershipId, WorkspaceResult,
};

/// Abstracts persistence for workspace membership assignments.
///
/// Use this port when a use case needs to answer who belongs to a workspace,
/// what role they hold there, or to persist membership changes without binding
/// the domain to a specific backing store.
pub trait MembershipRepository {
    /// Returns an error when the membership is absent.
    fn get(&self, membership_id: &WorkspaceMembershipId) -> WorkspaceResult<WorkspaceMembership>;

    /// Returns an error when the membership is absent for that workspace-user pair.
    fn find_for_workspace_user(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> WorkspaceResult<WorkspaceMembership>;

    /// Returns memberships in storage-defined order; callers must not rely on sorting.
    fn list_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<Vec<WorkspaceMembership>>;

    /// Saves the membership as an upsert.
    fn save(&self, membership: &WorkspaceMembership) -> WorkspaceResult<()>;
}
