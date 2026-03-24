use crate::workspace::WorkspacesReadPermission;

/// Represents a trusted internal caller whose service-to-service channel is
/// already verified outside actor-scoped admin authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationGuard {
    _verified: (),
}

impl IntegrationGuard {
    /// # Safety
    ///
    /// Callers must only mint this guard after verifying a trusted internal
    /// service-to-service request outside the domain layer.
    pub unsafe fn new_verified() -> Self {
        Self { _verified: () }
    }

    pub fn workspaces_read_permission(&self) -> WorkspacesReadPermission {
        WorkspacesReadPermission::new()
    }
}
