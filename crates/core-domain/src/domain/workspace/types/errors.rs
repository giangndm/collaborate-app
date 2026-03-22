use thiserror::Error;

use super::{UserId, WorkspaceApiKeyId, WorkspaceId, WorkspaceMembershipId};

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    #[error("workspace not found: {workspace_id}")]
    WorkspaceNotFound { workspace_id: WorkspaceId },
    #[error("user not found: {user_id}")]
    UserNotFound { user_id: UserId },
    #[error("workspace membership not found: {membership_id}")]
    MembershipNotFound {
        membership_id: WorkspaceMembershipId,
    },
    #[error("permission denied for user {user_id} on workspace {workspace_id}")]
    PermissionDenied {
        user_id: UserId,
        workspace_id: WorkspaceId,
    },
    #[error(
        "workspace permission mismatch: permission for {permission_workspace_id} cannot write target {target_workspace_id}"
    )]
    WorkspacePermissionMismatch {
        permission_workspace_id: WorkspaceId,
        target_workspace_id: WorkspaceId,
    },
    #[error("workspace credential not found: {api_key_id} in workspace {workspace_id}")]
    CredentialNotFound {
        api_key_id: WorkspaceApiKeyId,
        workspace_id: WorkspaceId,
    },
}
