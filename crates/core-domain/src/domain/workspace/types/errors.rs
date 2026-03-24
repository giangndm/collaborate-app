use thiserror::Error;

use super::{UserId, WorkspaceApiKeyId, WorkspaceId, WorkspaceMembershipId};

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    #[error("workspace not found: {workspace_id}")]
    WorkspaceNotFound { workspace_id: WorkspaceId },
    #[error("user not found: {user_id}")]
    UserNotFound { user_id: UserId },
    #[error("super admin role required for user {user_id}")]
    SuperAdminRequired { user_id: UserId },
    #[error("workspace membership not found: {membership_id}")]
    MembershipNotFound {
        membership_id: WorkspaceMembershipId,
    },
    #[error("workspace membership not found for user {user_id} in workspace {workspace_id}")]
    MembershipNotFoundForWorkspaceUser {
        workspace_id: WorkspaceId,
        user_id: UserId,
    },
    #[error("workspace member already exists for user {user_id} in workspace {workspace_id}")]
    MemberAlreadyExists {
        workspace_id: WorkspaceId,
        user_id: UserId,
    },
    #[error("permission denied for user {user_id} on workspace {workspace_id}")]
    PermissionDenied {
        user_id: UserId,
        workspace_id: WorkspaceId,
    },
    #[error("cannot remove the last owner {user_id} from workspace {workspace_id}")]
    LastOwnerRemovalDenied {
        workspace_id: WorkspaceId,
        user_id: UserId,
    },
    #[error("cannot demote the last owner {user_id} in workspace {workspace_id}")]
    LastOwnerDemotionDenied {
        workspace_id: WorkspaceId,
        user_id: UserId,
    },
    #[error(
        "owner {actor_user_id} cannot mutate peer owner {target_user_id} in workspace {workspace_id}"
    )]
    OwnerPeerMutationDenied {
        actor_user_id: UserId,
        target_user_id: UserId,
        workspace_id: WorkspaceId,
    },
    #[error(
        "only super admin can promote user {target_user_id} to owner in workspace {workspace_id}; actor was {actor_user_id}"
    )]
    OwnerPromotionRequiresSuperAdmin {
        actor_user_id: UserId,
        target_user_id: UserId,
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
