use crate::workspace::{
    GlobalUserRole, WorkspaceId, WorkspaceReadPermission, WorkspaceWritePermission,
};

/// Represents a caller whose global role has already been verified as super admin.
/// Use this guard when a workflow needs baseline cross-workspace access without coupling
/// permission derivation to user storage details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuperAdminGuard {
    _verified: (),
}

// Keep construction role-gated so callers cannot bypass verification.
// Add narrower super-admin scopes here if future policy splits read and write capabilities.

impl SuperAdminGuard {
    pub fn try_from_role(role: GlobalUserRole) -> Option<Self> {
        match role {
            GlobalUserRole::SuperAdmin => Some(Self { _verified: () }),
            GlobalUserRole::Member => None,
        }
    }

    pub fn read_permission(&self, workspace_id: WorkspaceId) -> WorkspaceReadPermission {
        WorkspaceReadPermission::new(workspace_id)
    }

    pub fn write_permission(&self, workspace_id: WorkspaceId) -> WorkspaceWritePermission {
        WorkspaceWritePermission::new(workspace_id)
    }
}
