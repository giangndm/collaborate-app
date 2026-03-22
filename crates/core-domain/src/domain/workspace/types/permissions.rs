use super::WorkspaceId;

/// Workspace read access minted by verified domain guards.
///
/// ```compile_fail
/// use core_domain::workspace::{WorkspaceId, WorkspaceReadPermission};
///
/// let workspace_id = WorkspaceId::from("ws_123".to_owned());
/// let _permission = WorkspaceReadPermission::new(workspace_id);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReadPermission {
    workspace_id: WorkspaceId,
}

impl WorkspaceReadPermission {
    pub(crate) fn new(workspace_id: WorkspaceId) -> Self {
        Self { workspace_id }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
}

/// Workspace write access minted by verified domain guards.
///
/// ```compile_fail
/// use core_domain::workspace::{WorkspaceId, WorkspaceWritePermission};
///
/// let workspace_id = WorkspaceId::from("ws_123".to_owned());
/// let _permission = WorkspaceWritePermission::new(workspace_id);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceWritePermission {
    workspace_id: WorkspaceId,
}

impl WorkspaceWritePermission {
    pub(crate) fn new(workspace_id: WorkspaceId) -> Self {
        Self { workspace_id }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
}
