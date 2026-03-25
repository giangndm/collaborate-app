use super::WorkspaceId;

/// Read-all workspace access minted by verified domain guards for sync flows.
///
/// Trusted integration guards should mint this only after verifying an
/// internal service-to-service request.
///
/// ```compile_fail
/// use core_domain::workspace::WorkspacesReadPermission;
///
/// let _permission = WorkspacesReadPermission::new();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkspacesReadPermission {
    _verified: (),
}

impl WorkspacesReadPermission {
    pub fn new() -> Self {
        Self { _verified: () }
    }
}

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
    pub fn new(workspace_id: WorkspaceId) -> Self {
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
    pub fn new(workspace_id: WorkspaceId) -> Self {
        Self { workspace_id }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
}
