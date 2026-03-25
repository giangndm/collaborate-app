use async_trait::async_trait;
use crate::workspace::{Workspace, WorkspaceId, WorkspaceMembership, WorkspaceResult};

/// Abstracts persistence for workspace aggregate state.
///
/// Use this port when a use case needs to load or persist the canonical
/// workspace entity without depending on storage details.
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    /// Returns an error when the workspace is absent.
    async fn get(&self, workspace_id: &WorkspaceId) -> WorkspaceResult<Workspace>;

    /// Returns all workspaces visible to cross-workspace admin flows.
    async fn list_all(&self) -> WorkspaceResult<Vec<Workspace>>;

    /// Returns the workspaces for the provided ids in storage-defined order.
    async fn list_for_ids(&self, workspace_ids: &[WorkspaceId]) -> WorkspaceResult<Vec<Workspace>>;

    /// Atomically creates the workspace together with its bootstrap owner membership.
    async fn create_with_owner(
        &self,
        workspace: &Workspace,
        owner_membership: &WorkspaceMembership,
    ) -> WorkspaceResult<()>;

    /// Saves the full aggregate state as an upsert.
    async fn save(&self, workspace: &Workspace) -> WorkspaceResult<()>;
}
