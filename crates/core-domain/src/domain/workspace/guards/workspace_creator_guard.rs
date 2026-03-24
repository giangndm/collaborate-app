use crate::workspace::{GlobalUserRole, User, UserId, UserStatus};

/// Represents a caller whose ability to bootstrap a brand new workspace is
/// already verified outside the workspace-scoped permission model.
///
/// Use this guard only for `WorkspaceService::create_workspace`, because that
/// flow runs before a concrete `workspace_id` exists and therefore cannot use
/// workspace-scoped read/write permissions yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCreatorGuard {
    actor_user_id: UserId,
}

impl WorkspaceCreatorGuard {
    pub fn try_from_actor(actor: &User) -> Option<Self> {
        match (actor.role(), actor.status()) {
            (GlobalUserRole::SuperAdmin, UserStatus::Active) => Some(Self {
                actor_user_id: actor.id().clone(),
            }),
            _ => None,
        }
    }

    pub fn actor_user_id(&self) -> &UserId {
        &self.actor_user_id
    }
}
