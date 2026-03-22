use crate::workspace::{
    WorkspaceId, WorkspaceReadPermission, WorkspaceRole, WorkspaceWritePermission,
};

/// Represents a caller whose membership in a specific workspace is already verified.
/// Use this guard after membership lookup has succeeded and later domain code only needs
/// baseline permission derivation instead of re-checking the role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMemberGuard {
    workspace_id: WorkspaceId,
    role: WorkspaceRole,
}

// Extend this guard if later business rules need explicit denial reasons or richer
// workspace-scoped actions.

impl WorkspaceMemberGuard {
    pub fn new(workspace_id: WorkspaceId, role: WorkspaceRole) -> Self {
        Self { workspace_id, role }
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn role(&self) -> WorkspaceRole {
        self.role
    }

    pub fn read_permission(&self) -> WorkspaceReadPermission {
        WorkspaceReadPermission::new(self.workspace_id.clone())
    }

    pub fn write_permission(&self) -> Option<WorkspaceWritePermission> {
        match self.role {
            WorkspaceRole::Owner | WorkspaceRole::Admin => {
                Some(WorkspaceWritePermission::new(self.workspace_id.clone()))
            }
            WorkspaceRole::Member => None,
        }
    }
}
