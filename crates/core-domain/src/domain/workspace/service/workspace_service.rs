use crate::workspace::{
    MembershipRepository, User, UserId, UserRepository, Workspace, WorkspaceCreatorGuard,
    WorkspaceError, WorkspaceMembership, WorkspaceReadPermission, WorkspaceRepository,
    WorkspaceResult, WorkspaceWritePermission,
};

/// Orchestrates the baseline workspace-domain use cases over typed ports.
///
/// This concrete service exists so callers can execute workspace reads and writes
/// through one small domain entry point while keeping permission checks and
/// persistence boundaries explicit in the type system.
///
/// Use this service when the workflow already holds a typed permission produced
/// by a guard and now needs to load or persist workspace state without pulling
/// adapter details into the domain layer.
#[derive(Debug)]
pub struct WorkspaceService<WorkspaceRepo, UserRepo, MembershipRepo> {
    pub(crate) workspace_repository: WorkspaceRepo,
    pub(crate) user_repository: UserRepo,
    pub(crate) membership_repository: MembershipRepo,
}

impl<WorkspaceRepo, UserRepo, MembershipRepo>
    WorkspaceService<WorkspaceRepo, UserRepo, MembershipRepo>
where
    WorkspaceRepo: WorkspaceRepository,
    UserRepo: UserRepository,
    MembershipRepo: MembershipRepository,
{
    /// Creates the service from typed repository contracts.
    pub fn new(
        workspace_repository: WorkspaceRepo,
        user_repository: UserRepo,
        membership_repository: MembershipRepo,
    ) -> Self {
        Self {
            workspace_repository,
            user_repository,
            membership_repository,
        }
    }

    /// Persists a newly prepared workspace aggregate under the creator bootstrap guard.
    pub fn create_workspace(
        &self,
        guard: &WorkspaceCreatorGuard,
        workspace: &Workspace,
    ) -> WorkspaceResult<()> {
        let _ = guard;
        self.workspace_repository.save(workspace)
    }

    /// Loads one workspace aggregate using workspace-scoped read permission.
    pub fn read_workspace(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<Workspace> {
        self.workspace_repository.get(permission.workspace_id())
    }

    /// Loads one member record and the corresponding user profile for the same
    /// workspace-scoped read request.
    pub fn read_member_user(
        &self,
        permission: &WorkspaceReadPermission,
        user_id: &UserId,
    ) -> WorkspaceResult<(WorkspaceMembership, User)> {
        // TODO(task-7): Add richer member-read orchestration here when the final
        // business flow is defined, such as filtering suspended users or joining
        // membership lifecycle state into a dedicated DTO.
        let membership = self
            .membership_repository
            .find_for_workspace_user(permission.workspace_id(), user_id)?;
        let user = self.user_repository.get(user_id)?;
        Ok((membership, user))
    }

    /// Lists memberships visible to a caller with workspace-scoped read access.
    pub fn list_members(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<Vec<WorkspaceMembership>> {
        self.membership_repository
            .list_for_workspace(permission.workspace_id())
    }

    /// Persists workspace aggregate updates under workspace-scoped write access.
    pub fn save_workspace(
        &self,
        permission: &WorkspaceWritePermission,
        workspace: &Workspace,
    ) -> WorkspaceResult<()> {
        ensure_write_target(permission, workspace.id())?;
        self.workspace_repository.save(workspace)
    }

    /// Persists membership changes under workspace-scoped write access.
    pub fn save_membership(
        &self,
        permission: &WorkspaceWritePermission,
        membership: &WorkspaceMembership,
    ) -> WorkspaceResult<()> {
        ensure_write_target(permission, membership.workspace_id())?;
        self.membership_repository.save(membership)
    }
}

fn ensure_write_target(
    permission: &WorkspaceWritePermission,
    target_workspace_id: &crate::workspace::WorkspaceId,
) -> WorkspaceResult<()> {
    if permission.workspace_id() == target_workspace_id {
        Ok(())
    } else {
        Err(WorkspaceError::WorkspacePermissionMismatch {
            permission_workspace_id: permission.workspace_id().clone(),
            target_workspace_id: target_workspace_id.clone(),
        })
    }
}
