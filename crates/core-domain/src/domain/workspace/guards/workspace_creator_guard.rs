/// Represents a caller whose ability to bootstrap a brand new workspace is
/// already verified outside the workspace-scoped permission model.
///
/// Use this guard only for `WorkspaceService::create_workspace`, because that
/// flow runs before a concrete `workspace_id` exists and therefore cannot use
/// workspace-scoped read/write permissions yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorkspaceCreatorGuard {
    _verified: (),
}

impl WorkspaceCreatorGuard {
    pub fn new() -> Self {
        Self { _verified: () }
    }
}
