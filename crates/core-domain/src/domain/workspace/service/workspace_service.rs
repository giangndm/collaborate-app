use std::collections::HashMap;

use crate::workspace::{
    GlobalUserRole, MembershipRepository, SecretStore, User, UserId, UserRepository, UserStatus,
    Workspace, WorkspaceApiKeyId, WorkspaceApiKeyMetadata, WorkspaceApiKeySecret,
    WorkspaceCreatorGuard, WorkspaceDetail, WorkspaceError, WorkspaceMemberView,
    WorkspaceMembership, WorkspaceReadPermission, WorkspaceRepository, WorkspaceResult,
    WorkspaceRole, WorkspaceSummary, WorkspaceUpdate, WorkspaceWritePermission,
};

/// Orchestrates workspace-domain use cases over typed ports.
#[derive(Debug)]
pub struct WorkspaceService<WorkspaceRepo, UserRepo, MembershipRepo, SecretStorePort> {
    pub(crate) workspace_repository: WorkspaceRepo,
    pub(crate) user_repository: UserRepo,
    pub(crate) membership_repository: MembershipRepo,
    pub(crate) secret_store: SecretStorePort,
}

impl<WorkspaceRepo, UserRepo, MembershipRepo, SecretStorePort>
    WorkspaceService<WorkspaceRepo, UserRepo, MembershipRepo, SecretStorePort>
where
    WorkspaceRepo: WorkspaceRepository,
    UserRepo: UserRepository,
    MembershipRepo: MembershipRepository,
    SecretStorePort: SecretStore,
{
    pub fn new(
        workspace_repository: WorkspaceRepo,
        user_repository: UserRepo,
        membership_repository: MembershipRepo,
        secret_store: SecretStorePort,
    ) -> Self {
        Self {
            workspace_repository,
            user_repository,
            membership_repository,
            secret_store,
        }
    }

    pub fn create_workspace(
        &self,
        guard: &WorkspaceCreatorGuard,
        workspace: &Workspace,
    ) -> WorkspaceResult<WorkspaceMembership> {
        let membership = WorkspaceMembership::new(
            bootstrap_membership_id(workspace, guard.actor_user_id()),
            workspace.id().clone(),
            guard.actor_user_id().clone(),
            WorkspaceRole::Owner,
        );
        self.workspace_repository
            .create_with_owner(workspace, &membership)?;

        Ok(membership)
    }

    pub fn list_workspaces_visible_to_actor(
        &self,
        actor: &User,
    ) -> WorkspaceResult<Vec<WorkspaceSummary>> {
        let workspaces = if actor.role() == GlobalUserRole::SuperAdmin {
            self.workspace_repository.list_all()?
        } else {
            let memberships = self.membership_repository.list_for_user(actor.id())?;
            let workspace_ids = memberships
                .into_iter()
                .map(|membership| membership.workspace_id().clone())
                .collect::<Vec<_>>();
            self.workspace_repository.list_for_ids(&workspace_ids)?
        };

        Ok(workspaces
            .iter()
            .map(WorkspaceSummary::from_workspace)
            .collect())
    }

    pub fn read_workspace(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<Workspace> {
        self.workspace_repository.get(permission.workspace_id())
    }

    pub fn get_workspace_detail(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<WorkspaceDetail> {
        let workspace = self.read_workspace(permission)?;
        Ok(WorkspaceDetail::from_workspace(&workspace))
    }

    pub fn list_credentials(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<Vec<WorkspaceApiKeyMetadata>> {
        self.workspace_repository.get(permission.workspace_id())?;
        self.secret_store.list_api_keys(permission.workspace_id())
    }

    pub fn create_credential(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        label: &str,
    ) -> WorkspaceResult<WorkspaceApiKeySecret> {
        let mut workspace = self.workspace_repository.get(permission.workspace_id())?;
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;
        ensure_write_target(permission, workspace.id())?;

        workspace.bump_last_updated();
        self.workspace_repository.save(&workspace)?;

        self.secret_store
            .create_api_key(permission.workspace_id(), label)
    }

    pub fn rotate_secret(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        api_key_id: &WorkspaceApiKeyId,
    ) -> WorkspaceResult<WorkspaceApiKeySecret> {
        let mut workspace = self.workspace_repository.get(permission.workspace_id())?;
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;
        ensure_write_target(permission, workspace.id())?;

        workspace.bump_last_updated();
        self.workspace_repository.save(&workspace)?;

        self.secret_store
            .rotate_api_key_secret(permission.workspace_id(), api_key_id)
    }

    pub fn update_workspace(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        update: WorkspaceUpdate,
    ) -> WorkspaceResult<WorkspaceDetail> {
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;

        let mut workspace = self.workspace_repository.get(permission.workspace_id())?;
        ensure_write_target(permission, workspace.id())?;
        workspace.apply_update(update);
        self.workspace_repository.save(&workspace)?;
        Ok(WorkspaceDetail::from_workspace(&workspace))
    }

    pub fn read_member_user(
        &self,
        permission: &WorkspaceReadPermission,
        user_id: &UserId,
    ) -> WorkspaceResult<(WorkspaceMembership, User)> {
        self.workspace_repository.get(permission.workspace_id())?;
        let membership = self
            .membership_repository
            .find_for_workspace_user(permission.workspace_id(), user_id)?;
        let user = self.user_repository.get(user_id)?;
        Ok((membership, user))
    }

    pub fn list_members(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<Vec<WorkspaceMemberView>> {
        self.workspace_repository.get(permission.workspace_id())?;
        let memberships = self
            .membership_repository
            .list_for_workspace(permission.workspace_id())?;
        let user_ids = memberships
            .iter()
            .map(|membership| membership.user_id().clone())
            .collect::<Vec<_>>();
        let users = self.user_repository.list_by_ids(&user_ids)?;
        let user_lookup = users
            .into_iter()
            .map(|user| (user.id().clone(), user))
            .collect::<HashMap<_, _>>();

        memberships
            .into_iter()
            .map(|membership| {
                let user = user_lookup.get(membership.user_id()).ok_or_else(|| {
                    WorkspaceError::UserNotFound {
                        user_id: membership.user_id().clone(),
                    }
                })?;

                Ok(WorkspaceMemberView::new(
                    membership.id().clone(),
                    membership.workspace_id().clone(),
                    user,
                    membership.role(),
                ))
            })
            .collect()
    }

    pub fn add_member(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        membership: &WorkspaceMembership,
    ) -> WorkspaceResult<WorkspaceMemberView> {
        self.workspace_repository.get(permission.workspace_id())?;
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;
        ensure_write_target(permission, membership.workspace_id())?;

        if membership.role() == WorkspaceRole::Owner && actor.role() != GlobalUserRole::SuperAdmin {
            return Err(WorkspaceError::OwnerPromotionRequiresSuperAdmin {
                actor_user_id: actor.id().clone(),
                target_user_id: membership.user_id().clone(),
                workspace_id: membership.workspace_id().clone(),
            });
        }

        match self
            .membership_repository
            .find_for_workspace_user(membership.workspace_id(), membership.user_id())
        {
            Ok(_) => {
                return Err(WorkspaceError::MemberAlreadyExists {
                    workspace_id: membership.workspace_id().clone(),
                    user_id: membership.user_id().clone(),
                });
            }
            Err(WorkspaceError::MembershipNotFoundForWorkspaceUser { .. }) => {}
            Err(error) => return Err(error),
        }

        let user = self.user_repository.get(membership.user_id())?;
        self.membership_repository
            .save_with_workspace_bump(membership)?;

        Ok(WorkspaceMemberView::new(
            membership.id().clone(),
            membership.workspace_id().clone(),
            &user,
            membership.role(),
        ))
    }

    pub fn remove_member(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        target_user_id: &UserId,
    ) -> WorkspaceResult<()> {
        self.workspace_repository.get(permission.workspace_id())?;
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;

        let membership = self
            .membership_repository
            .find_for_workspace_user(permission.workspace_id(), target_user_id)?;

        if membership.is_owner() {
            if count_workspace_owners(&self.membership_repository, permission.workspace_id())? == 1
            {
                return Err(WorkspaceError::LastOwnerRemovalDenied {
                    workspace_id: permission.workspace_id().clone(),
                    user_id: target_user_id.clone(),
                });
            }

            if actor.role() != GlobalUserRole::SuperAdmin {
                return Err(WorkspaceError::OwnerPeerMutationDenied {
                    actor_user_id: actor.id().clone(),
                    target_user_id: target_user_id.clone(),
                    workspace_id: permission.workspace_id().clone(),
                });
            }
        }

        self.membership_repository
            .remove_with_workspace_bump(permission.workspace_id(), membership.id())?;
        Ok(())
    }

    pub fn change_member_role(
        &self,
        permission: &WorkspaceWritePermission,
        actor: &User,
        target_user_id: &UserId,
        role: WorkspaceRole,
    ) -> WorkspaceResult<WorkspaceMemberView> {
        self.workspace_repository.get(permission.workspace_id())?;
        authorize_workspace_mutation(
            &self.membership_repository,
            permission.workspace_id(),
            actor,
        )?;

        let mut membership = self
            .membership_repository
            .find_for_workspace_user(permission.workspace_id(), target_user_id)?;

        if role == WorkspaceRole::Owner && actor.role() != GlobalUserRole::SuperAdmin {
            return Err(WorkspaceError::OwnerPromotionRequiresSuperAdmin {
                actor_user_id: actor.id().clone(),
                target_user_id: target_user_id.clone(),
                workspace_id: permission.workspace_id().clone(),
            });
        }

        if membership.is_owner() && role != WorkspaceRole::Owner {
            if count_workspace_owners(&self.membership_repository, permission.workspace_id())? == 1
            {
                return Err(WorkspaceError::LastOwnerDemotionDenied {
                    workspace_id: permission.workspace_id().clone(),
                    user_id: target_user_id.clone(),
                });
            }

            if actor.role() != GlobalUserRole::SuperAdmin {
                return Err(WorkspaceError::OwnerPeerMutationDenied {
                    actor_user_id: actor.id().clone(),
                    target_user_id: target_user_id.clone(),
                    workspace_id: permission.workspace_id().clone(),
                });
            }
        }

        let user = self.user_repository.get(target_user_id)?;
        membership.change_role(role);
        self.membership_repository
            .save_with_workspace_bump(&membership)?;

        Ok(WorkspaceMemberView::new(
            membership.id().clone(),
            membership.workspace_id().clone(),
            &user,
            membership.role(),
        ))
    }

    pub fn save_workspace(
        &self,
        permission: &WorkspaceWritePermission,
        workspace: &Workspace,
    ) -> WorkspaceResult<()> {
        ensure_write_target(permission, workspace.id())?;
        self.workspace_repository.save(workspace)
    }

    pub fn save_membership(
        &self,
        permission: &WorkspaceWritePermission,
        membership: &WorkspaceMembership,
    ) -> WorkspaceResult<()> {
        ensure_write_target(permission, membership.workspace_id())?;
        self.membership_repository
            .save_with_workspace_bump(membership)
    }
}

fn bootstrap_membership_id(
    workspace: &Workspace,
    user_id: &UserId,
) -> crate::workspace::WorkspaceMembershipId {
    crate::workspace::WorkspaceMembershipId::new(format!(
        "{}:{}",
        workspace.id().as_str(),
        user_id.as_str()
    ))
}

fn authorize_workspace_mutation<MembershipRepo>(
    membership_repository: &MembershipRepo,
    workspace_id: &crate::workspace::WorkspaceId,
    actor: &User,
) -> WorkspaceResult<()>
where
    MembershipRepo: MembershipRepository,
{
    if actor.status() != UserStatus::Active {
        return Err(WorkspaceError::PermissionDenied {
            user_id: actor.id().clone(),
            workspace_id: workspace_id.clone(),
        });
    }

    if actor.role() == GlobalUserRole::SuperAdmin {
        return Ok(());
    }

    let membership = membership_repository.find_for_workspace_user(workspace_id, actor.id())?;
    if membership.role() == WorkspaceRole::Owner {
        Ok(())
    } else {
        Err(WorkspaceError::PermissionDenied {
            user_id: actor.id().clone(),
            workspace_id: workspace_id.clone(),
        })
    }
}

fn count_workspace_owners<MembershipRepo>(
    membership_repository: &MembershipRepo,
    workspace_id: &crate::workspace::WorkspaceId,
) -> WorkspaceResult<usize>
where
    MembershipRepo: MembershipRepository,
{
    Ok(membership_repository
        .list_for_workspace(workspace_id)?
        .into_iter()
        .filter(|membership| membership.is_owner())
        .count())
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
