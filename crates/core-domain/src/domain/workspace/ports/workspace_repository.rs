use crate::workspace::{Workspace, WorkspaceId, WorkspaceResult};

/// Abstracts persistence for workspace aggregate state.
///
/// Use this port when a use case needs to load or persist the canonical
/// workspace entity without depending on storage details.
pub trait WorkspaceRepository {
    /// Returns an error when the workspace is absent.
    fn get(&self, workspace_id: &WorkspaceId) -> WorkspaceResult<Workspace>;

    /// Saves the full aggregate state as an upsert.
    fn save(&self, workspace: &Workspace) -> WorkspaceResult<()>;
}
