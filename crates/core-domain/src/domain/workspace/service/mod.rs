//! Workspace-domain orchestration services live here.

mod sync_service;
mod workspace_service;

pub use sync_service::*;
pub use workspace_service::*;

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::{WorkspaceService, WorkspaceSyncService};
    use crate::workspace::{
        DefaultRoomPolicy, DisplayName, GlobalUserRole, GuestAccessPolicy, IntegrationGuard,
        MembershipRepository, SecretStore, User, UserEmail, UserId, UserProfile, UserRepository,
        Workspace, WorkspaceApiKeyId, WorkspaceApiKeyMetadata, WorkspaceApiKeySecret,
        WorkspaceCreatorGuard, WorkspaceCredentialStatus, WorkspaceCredentialStatus,
        WorkspaceDetail, WorkspaceDetail, WorkspaceError, WorkspaceError, WorkspaceId, WorkspaceId,
        WorkspaceLastUpdated, WorkspaceMemberGuard, WorkspaceMemberView, WorkspaceMembership,
        WorkspaceMembershipId, WorkspaceName, WorkspacePolicy, WorkspaceRepository,
        WorkspaceResult, WorkspaceRole, WorkspaceSecretRef, WorkspaceSecretRefId,
        WorkspaceSecretVersion, WorkspaceSigningProfile, WorkspaceSlug, WorkspaceStatus,
        WorkspaceSummary, WorkspaceUpdate,
    };

    fn verified_member_guard(
        workspace_id: WorkspaceId,
        role: WorkspaceRole,
    ) -> WorkspaceMemberGuard {
        unsafe { WorkspaceMemberGuard::new_verified(workspace_id, role) }
    }

    #[test]
    fn workspace_service_orchestrates_basic_workspace_reads_and_writes() {
        let workspace = sample_workspace("ws_service");
        let membership =
            sample_membership("membership_service", workspace.id().clone(), "user_service");
        let user = sample_user_with_role("user_service", GlobalUserRole::SuperAdmin);
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository = RecordingMembershipRepository::new(membership.clone());
        let user_repository = RecordingUserRepository::new(user.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let creator_guard = WorkspaceCreatorGuard::try_from_actor(&user)
            .expect("super admin role should mint creator guard");
        let read_permission =
            verified_member_guard(workspace.id().clone(), WorkspaceRole::Member).read_permission();
        let write_permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write access");

        service
            .create_workspace(&creator_guard, &workspace)
            .expect("create workspace should delegate to repository save");
        let loaded_workspace = service
            .read_workspace(&read_permission)
            .expect("read workspace should delegate to repository get");
        let (loaded_membership, loaded_user) = service
            .read_member_user(&read_permission, user.id())
            .expect("member user read should compose membership and user repositories");
        let listed_memberships = service
            .list_members(&read_permission)
            .expect("member list should use workspace-scoped read permission");
        service
            .save_workspace(&write_permission, &workspace)
            .expect("save workspace should delegate to repository save");
        service
            .save_membership(&write_permission, &membership)
            .expect("save membership should delegate to repository save");

        assert_eq!(loaded_workspace, workspace);
        assert_eq!(loaded_membership, membership);
        assert_eq!(loaded_user, user);
        assert_eq!(
            listed_memberships,
            vec![WorkspaceMemberView::new(
                membership.id().clone(),
                workspace.id().clone(),
                &user,
                membership.role(),
            )]
        );
        assert_eq!(
            service
                .workspace_repository
                .recorded_gets
                .borrow()
                .as_slice(),
            &[
                workspace.id().clone(),
                workspace.id().clone(),
                workspace.id().clone(),
            ]
        );
        assert_eq!(
            service
                .workspace_repository
                .recorded_bootstrap_creates
                .borrow()
                .as_slice(),
            &[(
                workspace.id().clone(),
                WorkspaceMembershipId::new(format!(
                    "{}:{}",
                    workspace.id().as_str(),
                    user.id().as_str()
                ))
            )]
        );
        assert_eq!(
            service
                .workspace_repository
                .recorded_saves
                .borrow()
                .as_slice(),
            &[workspace.id().clone()]
        );
        assert_eq!(
            service
                .membership_repository
                .recorded_find_for_workspace_user
                .borrow()
                .as_slice(),
            &[(workspace.id().clone(), user.id().clone())]
        );
        assert_eq!(
            service
                .membership_repository
                .recorded_list_for_workspace
                .borrow()
                .as_slice(),
            &[workspace.id().clone()]
        );
        assert_eq!(
            service
                .membership_repository
                .recorded_saves
                .borrow()
                .as_slice(),
            &[]
        );
        let recorded_atomic_saves = service.membership_repository.recorded_atomic_saves.borrow();
        assert_eq!(recorded_atomic_saves.len(), 1);
        assert_eq!(recorded_atomic_saves[0].0, membership.id().clone());
        assert_eq!(
            service.user_repository.recorded_gets.borrow().as_slice(),
            &[user.id().clone()]
        );
    }

    #[test]
    fn workspace_sync_service_exports_payload_from_workspace_and_secret_contracts() {
        let workspace = sample_workspace("ws_sync");
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let secret_store =
            RecordingSecretStore::new(vec![sample_api_key("api_sync", "secret_api_sync", 9)]);
        let service = WorkspaceSyncService::new(workspace_repository, secret_store);
        let permission =
            verified_member_guard(workspace.id().clone(), WorkspaceRole::Member).read_permission();

        let payload = service
            .export_sync_payload(&permission)
            .expect("sync payload export should compose repository and secret-store state");

        assert_eq!(payload.workspace_id, workspace.id().clone());
        assert_eq!(payload.status, WorkspaceStatus::Active);
        assert_eq!(payload.last_updated, workspace.last_updated());
        assert_eq!(
            payload.default_room_policy,
            workspace.default_room_policy().clone()
        );
        assert_eq!(
            payload.credential_verifiers,
            vec![crate::workspace::WorkspaceCredentialVerifier {
                api_key_id: WorkspaceApiKeyId::new("api_sync"),
                secret_ref_id: WorkspaceSecretRefId::new("secret_api_sync"),
                version: WorkspaceSecretVersion::new(9),
                status: WorkspaceCredentialStatus::Active,
            }]
        );
        assert_eq!(
            service
                .workspace_repository
                .recorded_gets
                .borrow()
                .as_slice(),
            &[workspace.id().clone()]
        );
        assert_eq!(
            service
                .secret_store
                .recorded_api_key_reads
                .borrow()
                .as_slice(),
            &[workspace.id().clone()]
        );
    }

    #[test]
    fn workspace_service_rejects_workspace_write_permission_for_other_workspace() {
        let workspace = sample_workspace("ws_target");
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository = RecordingMembershipRepository::new(sample_membership(
            "membership_target",
            workspace.id().clone(),
            "user_target",
        ));
        let user_repository = RecordingUserRepository::new(sample_user("user_target"));
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let wrong_permission =
            verified_member_guard(WorkspaceId::new("ws_other"), WorkspaceRole::Owner)
                .write_permission()
                .expect("owner should derive write access");

        let error = service
            .save_workspace(&wrong_permission, &workspace)
            .expect_err("mismatched workspace write permission should be rejected");

        assert_eq!(
            error,
            WorkspaceError::WorkspacePermissionMismatch {
                permission_workspace_id: WorkspaceId::new("ws_other"),
                target_workspace_id: workspace.id().clone(),
            }
        );
        assert!(service
            .workspace_repository
            .recorded_saves
            .borrow()
            .is_empty());
    }

    #[test]
    fn workspace_service_rejects_membership_write_permission_for_other_workspace() {
        let workspace = sample_workspace("ws_target_membership");
        let membership = sample_membership(
            "membership_target",
            workspace.id().clone(),
            "user_target_membership",
        );
        let workspace_repository = RecordingWorkspaceRepository::new(workspace);
        let membership_repository = RecordingMembershipRepository::new(membership.clone());
        let user_repository = RecordingUserRepository::new(sample_user("user_target_membership"));
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let wrong_permission = verified_member_guard(
            WorkspaceId::new("ws_other_membership"),
            WorkspaceRole::Owner,
        )
        .write_permission()
        .expect("owner should derive write access");

        let error = service
            .save_membership(&wrong_permission, &membership)
            .expect_err("mismatched membership write permission should be rejected");

        assert_eq!(
            error,
            WorkspaceError::WorkspacePermissionMismatch {
                permission_workspace_id: WorkspaceId::new("ws_other_membership"),
                target_workspace_id: membership.workspace_id().clone(),
            }
        );
        assert!(service
            .membership_repository
            .recorded_saves
            .borrow()
            .is_empty());
    }

    #[test]
    fn non_super_admin_cannot_create_workspace_via_creator_guard() {
        let actor = sample_user_with_role("user_member_creator", GlobalUserRole::Member);

        assert!(WorkspaceCreatorGuard::try_from_actor(&actor).is_none());
    }

    #[test]
    fn create_workspace_bootstraps_first_owner() {
        let actor = sample_user_with_role("user_super_admin", GlobalUserRole::SuperAdmin);
        let workspace = workspace_with_room_policy("ws_create", false, 3600);
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository = RecordingMembershipRepository::empty();
        let user_repository = RecordingUserRepository::new(actor.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);

        let creator_guard = WorkspaceCreatorGuard::try_from_actor(&actor)
            .expect("super admin role should mint creator guard");

        let membership = service
            .create_workspace(&creator_guard, &workspace)
            .expect("create workspace should persist the bootstrap owner membership");

        assert_eq!(membership.workspace_id(), workspace.id());
        assert_eq!(membership.user_id(), actor.id());
        assert_eq!(membership.role(), WorkspaceRole::Owner);
        assert_eq!(
            service
                .workspace_repository
                .recorded_bootstrap_creates
                .borrow()
                .as_slice(),
            &[(workspace.id().clone(), membership.id().clone())]
        );
        assert!(service
            .workspace_repository
            .recorded_saves
            .borrow()
            .is_empty());
        assert!(service
            .membership_repository
            .recorded_saves
            .borrow()
            .is_empty());
    }

    #[test]
    fn list_workspaces_visible_to_actor_returns_all_for_super_admin() {
        let actor = sample_user_with_role("user_super_admin_list", GlobalUserRole::SuperAdmin);
        let alpha = workspace_with_room_policy("ws_alpha_visible", false, 3600);
        let beta = workspace_with_room_policy("ws_beta_visible", true, 1800);
        let workspace_repository =
            RecordingWorkspaceRepository::with_workspaces(vec![alpha.clone(), beta.clone()]);
        let membership_repository = RecordingMembershipRepository::empty();
        let user_repository = RecordingUserRepository::new(actor.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);

        let visible = service
            .list_workspaces_visible_to_actor(&actor)
            .expect("super admin should see all workspaces");

        assert_eq!(
            visible,
            vec![
                WorkspaceSummary::from_workspace(&alpha),
                WorkspaceSummary::from_workspace(&beta),
            ]
        );
        assert_eq!(service.workspace_repository.recorded_list_all.get(), 1);
        assert!(service
            .membership_repository
            .recorded_list_for_user
            .borrow()
            .is_empty());
    }

    #[test]
    fn list_workspaces_visible_to_actor_returns_only_memberships_for_member() {
        let actor = sample_user_with_role("user_member_list", GlobalUserRole::Member);
        let alpha = workspace_with_room_policy("ws_member_alpha", false, 3600);
        let beta = workspace_with_room_policy("ws_member_beta", true, 1800);
        let workspace_repository =
            RecordingWorkspaceRepository::with_workspaces(vec![alpha.clone(), beta.clone()]);
        let membership_repository =
            RecordingMembershipRepository::with_memberships(vec![WorkspaceMembership::new(
                WorkspaceMembershipId::new("membership_alpha"),
                alpha.id().clone(),
                actor.id().clone(),
                WorkspaceRole::Member,
            )]);
        let user_repository = RecordingUserRepository::new(actor.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);

        let visible = service
            .list_workspaces_visible_to_actor(&actor)
            .expect("member should only see their memberships");

        assert_eq!(visible, vec![WorkspaceSummary::from_workspace(&alpha)]);
        assert_eq!(
            service
                .membership_repository
                .recorded_list_for_user
                .borrow()
                .as_slice(),
            &[actor.id().clone()]
        );
        assert_eq!(
            service
                .workspace_repository
                .recorded_list_for_ids
                .borrow()
                .as_slice(),
            &[vec![alpha.id().clone()]]
        );
    }

    #[test]
    fn get_workspace_detail_returns_workspace_with_policy() {
        let workspace = workspace_with_room_policy("ws_detail", true, 900);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::new(sample_user("user_detail")),
            RecordingMembershipRepository::empty(),
        );
        let permission =
            verified_member_guard(workspace.id().clone(), WorkspaceRole::Member).read_permission();

        let detail = service
            .get_workspace_detail(&permission)
            .expect("detail should include default room policy");

        assert_eq!(detail, WorkspaceDetail::from_workspace(&workspace));
        assert_eq!(detail.name, WorkspaceName::new("Workspace ws_detail"));
        assert_eq!(detail.slug, WorkspaceSlug::new("ws-detail"));
    }

    #[test]
    fn update_workspace_updates_default_room_policy() {
        let actor = sample_user_with_role("user_owner_update", GlobalUserRole::Member);
        let workspace = workspace_with_room_policy("ws_update", false, 3600);
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository =
            RecordingMembershipRepository::with_memberships(vec![WorkspaceMembership::new(
                WorkspaceMembershipId::new("membership_owner_update"),
                workspace.id().clone(),
                actor.id().clone(),
                WorkspaceRole::Owner,
            )]);
        let user_repository = RecordingUserRepository::new(actor.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let detail = service
            .update_workspace(
                &permission,
                &actor,
                WorkspaceUpdate::new(
                    WorkspaceName::new("Workspace ws_update_renamed"),
                    WorkspaceStatus::Suspended,
                    DefaultRoomPolicy::new(true, 1800),
                ),
            )
            .expect("owner update should persist workspace metadata status and room policy");

        assert_eq!(
            detail.name,
            WorkspaceName::new("Workspace ws_update_renamed")
        );
        assert_eq!(detail.slug, WorkspaceSlug::new("ws-update"));
        assert_eq!(detail.status, WorkspaceStatus::Suspended);
        assert_eq!(detail.policy.guest_access, GuestAccessPolicy::Allowed);
        assert_eq!(
            detail.default_room_policy,
            DefaultRoomPolicy::new(true, 1800)
        );
        assert_eq!(
            service
                .workspace_repository
                .saved_workspaces
                .borrow()
                .last()
                .expect("saved workspace should be recorded")
                .name(),
            &WorkspaceName::new("Workspace ws_update_renamed")
        );
        assert_eq!(
            service
                .workspace_repository
                .saved_workspaces
                .borrow()
                .last()
                .expect("saved workspace should be recorded")
                .slug(),
            &WorkspaceSlug::new("ws-update")
        );
        assert_eq!(
            service
                .workspace_repository
                .saved_workspaces
                .borrow()
                .last()
                .expect("saved workspace should be recorded")
                .status(),
            WorkspaceStatus::Suspended
        );
        assert_eq!(
            service
                .workspace_repository
                .saved_workspaces
                .borrow()
                .last()
                .expect("saved workspace should be recorded")
                .policy()
                .guest_access,
            GuestAccessPolicy::Allowed
        );
        assert_eq!(
            service
                .workspace_repository
                .saved_workspaces
                .borrow()
                .last()
                .expect("saved workspace should be recorded")
                .default_room_policy(),
            &DefaultRoomPolicy::new(true, 1800)
        );
    }

    #[test]
    fn list_members_returns_workspace_members() {
        let workspace = workspace_with_room_policy("ws_members", false, 3600);
        let owner = sample_user("user_owner_members");
        let member = sample_user("user_member_members");
        let permission =
            verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner).read_permission();
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![owner.clone(), member.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_members"),
                    workspace.id().clone(),
                    owner.id().clone(),
                    WorkspaceRole::Owner,
                ),
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_member_members"),
                    workspace.id().clone(),
                    member.id().clone(),
                    WorkspaceRole::Member,
                ),
            ]),
        );

        let members = service
            .list_members(&permission)
            .expect("member list should join membership and user data");

        assert_eq!(members.len(), 2);
        assert_eq!(
            members[0],
            WorkspaceMemberView::new(
                WorkspaceMembershipId::new("membership_owner_members"),
                workspace.id().clone(),
                &owner,
                WorkspaceRole::Owner,
            )
        );
        assert_eq!(
            members[1],
            WorkspaceMemberView::new(
                WorkspaceMembershipId::new("membership_member_members"),
                workspace.id().clone(),
                &member,
                WorkspaceRole::Member,
            )
        );
    }

    #[test]
    fn add_member_returns_member_already_exists_for_duplicate() {
        let actor = sample_user("user_owner_add_duplicate");
        let workspace = workspace_with_room_policy("ws_add_duplicate", false, 3600);
        let duplicate = sample_user("user_duplicate");
        let membership = WorkspaceMembership::new(
            WorkspaceMembershipId::new("membership_duplicate"),
            workspace.id().clone(),
            duplicate.id().clone(),
            WorkspaceRole::Member,
        );
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![actor.clone(), duplicate.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_add_duplicate"),
                    workspace.id().clone(),
                    actor.id().clone(),
                    WorkspaceRole::Owner,
                ),
                membership.clone(),
            ]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .add_member(&permission, &actor, &membership)
            .expect_err("duplicate member add should be rejected");

        assert_eq!(
            error,
            WorkspaceError::MemberAlreadyExists {
                workspace_id: workspace.id().clone(),
                user_id: duplicate.id().clone(),
            }
        );
    }

    #[test]
    fn remove_member_rejects_removing_last_owner() {
        let actor =
            sample_user_with_role("user_super_admin_remove_last", GlobalUserRole::SuperAdmin);
        let workspace = workspace_with_room_policy("ws_remove_last_owner", false, 3600);
        let owner_membership = WorkspaceMembership::new(
            WorkspaceMembershipId::new("membership_remove_last_owner"),
            workspace.id().clone(),
            actor.id().clone(),
            WorkspaceRole::Owner,
        );
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::new(actor.clone()),
            RecordingMembershipRepository::with_memberships(vec![owner_membership.clone()]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .remove_member(&permission, &actor, actor.id())
            .expect_err("last owner removal should be rejected");

        assert_eq!(
            error,
            WorkspaceError::LastOwnerRemovalDenied {
                workspace_id: workspace.id().clone(),
                user_id: actor.id().clone(),
            }
        );
    }

    #[test]
    fn owner_cannot_remove_another_owner() {
        let actor = sample_user("user_owner_remove_actor");
        let target = sample_user("user_owner_remove_target");
        let workspace = workspace_with_room_policy("ws_owner_remove", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![actor.clone(), target.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_remove_actor"),
                    workspace.id().clone(),
                    actor.id().clone(),
                    WorkspaceRole::Owner,
                ),
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_remove_target"),
                    workspace.id().clone(),
                    target.id().clone(),
                    WorkspaceRole::Owner,
                ),
            ]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .remove_member(&permission, &actor, target.id())
            .expect_err("owner should not remove another owner");

        assert_eq!(
            error,
            WorkspaceError::OwnerPeerMutationDenied {
                actor_user_id: actor.id().clone(),
                target_user_id: target.id().clone(),
                workspace_id: workspace.id().clone(),
            }
        );
    }

    #[test]
    fn change_member_role_updates_member_to_member_only() {
        let actor =
            sample_user_with_role("user_super_admin_change_role", GlobalUserRole::SuperAdmin);
        let target = sample_user("user_owner_demote");
        let workspace = workspace_with_room_policy("ws_change_role", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![actor.clone(), target.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_demote"),
                    workspace.id().clone(),
                    target.id().clone(),
                    WorkspaceRole::Owner,
                ),
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_demote_peer"),
                    workspace.id().clone(),
                    actor.id().clone(),
                    WorkspaceRole::Owner,
                ),
            ]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let member = service
            .change_member_role(&permission, &actor, target.id(), WorkspaceRole::Member)
            .expect("super admin should be able to demote a non-last owner");

        assert_eq!(member.workspace_role, WorkspaceRole::Member);
    }

    #[test]
    fn change_member_role_rejects_last_owner_demotion() {
        let actor = sample_user_with_role("user_super_admin_demotion", GlobalUserRole::SuperAdmin);
        let workspace = workspace_with_room_policy("ws_demotion_last_owner", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::new(actor.clone()),
            RecordingMembershipRepository::with_memberships(vec![WorkspaceMembership::new(
                WorkspaceMembershipId::new("membership_last_owner_demotion"),
                workspace.id().clone(),
                actor.id().clone(),
                WorkspaceRole::Owner,
            )]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .change_member_role(&permission, &actor, actor.id(), WorkspaceRole::Member)
            .expect_err("last owner demotion should be rejected");

        assert_eq!(
            error,
            WorkspaceError::LastOwnerDemotionDenied {
                workspace_id: workspace.id().clone(),
                user_id: actor.id().clone(),
            }
        );
    }

    #[test]
    fn only_super_admin_can_promote_member_to_owner() {
        let actor = sample_user("user_owner_promote_denied");
        let target = sample_user("user_member_promote_denied");
        let workspace = workspace_with_room_policy("ws_promote_denied", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![actor.clone(), target.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_promote_denied"),
                    workspace.id().clone(),
                    actor.id().clone(),
                    WorkspaceRole::Owner,
                ),
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_member_promote_denied"),
                    workspace.id().clone(),
                    target.id().clone(),
                    WorkspaceRole::Member,
                ),
            ]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .change_member_role(&permission, &actor, target.id(), WorkspaceRole::Owner)
            .expect_err("owner should not promote another member to owner");

        assert_eq!(
            error,
            WorkspaceError::OwnerPromotionRequiresSuperAdmin {
                actor_user_id: actor.id().clone(),
                target_user_id: target.id().clone(),
                workspace_id: workspace.id().clone(),
            }
        );
    }

    #[test]
    fn owner_cannot_create_another_owner_via_role_change() {
        let actor = sample_user("user_owner_create_owner_denied");
        let target = sample_user("user_member_create_owner_denied");
        let workspace = workspace_with_room_policy("ws_owner_create_owner", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::with_users(vec![actor.clone(), target.clone()]),
            RecordingMembershipRepository::with_memberships(vec![
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_create_owner_actor"),
                    workspace.id().clone(),
                    actor.id().clone(),
                    WorkspaceRole::Owner,
                ),
                WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_owner_create_owner_target"),
                    workspace.id().clone(),
                    target.id().clone(),
                    WorkspaceRole::Member,
                ),
            ]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");

        let error = service
            .change_member_role(&permission, &actor, target.id(), WorkspaceRole::Owner)
            .expect_err("owner should not create another owner");

        assert_eq!(
            error,
            WorkspaceError::OwnerPromotionRequiresSuperAdmin {
                actor_user_id: actor.id().clone(),
                target_user_id: target.id().clone(),
                workspace_id: workspace.id().clone(),
            }
        );
    }

    #[test]
    fn member_cannot_mutate_workspace() {
        let actor = sample_user("user_member_mutation_denied");
        let workspace = workspace_with_room_policy("ws_member_mutation_denied", false, 3600);
        let service = WorkspaceService::new(
            RecordingWorkspaceRepository::new(workspace.clone()),
            RecordingUserRepository::new(actor.clone()),
            RecordingMembershipRepository::with_memberships(vec![WorkspaceMembership::new(
                WorkspaceMembershipId::new("membership_member_mutation_denied"),
                workspace.id().clone(),
                actor.id().clone(),
                WorkspaceRole::Member,
            )]),
        );
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("typed permission alone should not authorize a member actor");

        let error = service
            .update_workspace(
                &permission,
                &actor,
                WorkspaceUpdate::new(
                    WorkspaceName::new("Workspace ws_member_mutation_denied"),
                    WorkspaceStatus::Active,
                    DefaultRoomPolicy::new(true, 1200),
                ),
            )
            .expect_err("member actor should not mutate workspace state");

        assert_eq!(
            error,
            WorkspaceError::PermissionDenied {
                user_id: actor.id().clone(),
                workspace_id: workspace.id().clone(),
            }
        );
    }

    #[test]
    fn workspace_mutations_bump_last_updated() {
        let actor = sample_user("user_owner_last_updated");
        let target = sample_user("user_member_last_updated");
        let workspace = workspace_with_room_policy("ws_last_updated", false, 3600);
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository =
            RecordingMembershipRepository::with_memberships(vec![WorkspaceMembership::new(
                WorkspaceMembershipId::new("membership_owner_last_updated"),
                workspace.id().clone(),
                actor.id().clone(),
                WorkspaceRole::Owner,
            )]);
        let user_repository =
            RecordingUserRepository::with_users(vec![actor.clone(), target.clone()]);
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let permission = verified_member_guard(workspace.id().clone(), WorkspaceRole::Owner)
            .write_permission()
            .expect("owner should derive write permission");
        let before = workspace.last_updated();

        let updated_workspace = service
            .update_workspace(
                &permission,
                &actor,
                WorkspaceUpdate::new(
                    WorkspaceName::new("Workspace ws_last_updated_renamed"),
                    WorkspaceStatus::Suspended,
                    DefaultRoomPolicy::new(true, 1800),
                ),
            )
            .expect("workspace update should succeed");
        let after_workspace_update = updated_workspace.last_updated;
        let _member = service
            .add_member(
                &permission,
                &actor,
                &WorkspaceMembership::new(
                    WorkspaceMembershipId::new("membership_added_last_updated"),
                    workspace.id().clone(),
                    target.id().clone(),
                    WorkspaceRole::Member,
                ),
            )
            .expect("member add should succeed");
        let after_member_add = service
            .membership_repository
            .recorded_atomic_saves
            .borrow()
            .last()
            .expect("atomic membership save after add should be recorded")
            .2
            .clone();

        assert!(after_workspace_update > before);
        assert!(after_member_add > before);
    }

    fn sample_workspace(value: &str) -> Workspace {
        Workspace::new(
            WorkspaceId::new(value),
            WorkspaceName::new(format!("Workspace {value}")),
            WorkspaceSlug::new(value.replace('_', "-")),
            WorkspaceStatus::Active,
            WorkspacePolicy {
                guest_access: GuestAccessPolicy::Allowed,
            },
            DefaultRoomPolicy::new(false, 3600),
            sample_signing_profile("workspace_signing", 3),
        )
    }

    fn sample_user(value: &str) -> User {
        sample_user_with_role(value, GlobalUserRole::Member)
    }

    fn sample_user_with_role(value: &str, role: GlobalUserRole) -> User {
        User::new(
            UserId::new(value),
            role,
            UserProfile::new(
                UserEmail::new(format!("{value}@example.com")),
                DisplayName::new(format!("User {value}")),
            ),
        )
    }

    fn workspace_with_room_policy(
        value: &str,
        guest_join_enabled: bool,
        token_ttl_seconds: u32,
    ) -> Workspace {
        Workspace::new(
            WorkspaceId::new(value),
            WorkspaceName::new(format!("Workspace {value}")),
            WorkspaceSlug::new(value.replace('_', "-")),
            WorkspaceStatus::Active,
            WorkspacePolicy {
                guest_access: GuestAccessPolicy::Allowed,
            },
            DefaultRoomPolicy::new(guest_join_enabled, token_ttl_seconds),
            sample_signing_profile("workspace_signing", 3),
        )
    }

    fn sample_membership(
        membership_id: &str,
        workspace_id: WorkspaceId,
        user_id: &str,
    ) -> WorkspaceMembership {
        WorkspaceMembership::new(
            WorkspaceMembershipId::new(membership_id),
            workspace_id,
            UserId::new(user_id),
            WorkspaceRole::Member,
        )
    }

    fn sample_signing_profile(secret_ref_id: &str, version: u64) -> WorkspaceSigningProfile {
        WorkspaceSigningProfile {
            active_secret_ref: WorkspaceSecretRef {
                secret_ref_id: WorkspaceSecretRefId::new(secret_ref_id),
                version: WorkspaceSecretVersion::new(version),
            },
        }
    }

    fn sample_api_key(
        api_key_id: &str,
        secret_ref_id: &str,
        version: u64,
    ) -> WorkspaceApiKeyMetadata {
        WorkspaceApiKeyMetadata {
            api_key_id: WorkspaceApiKeyId::new(api_key_id),
            secret_ref: WorkspaceSecretRef {
                secret_ref_id: WorkspaceSecretRefId::new(secret_ref_id),
                version: WorkspaceSecretVersion::new(version),
            },
            status: WorkspaceCredentialStatus::Active,
        }
    }

    #[derive(Debug)]
    struct RecordingWorkspaceRepository {
        workspaces: RefCell<Vec<Workspace>>,
        recorded_gets: RefCell<Vec<WorkspaceId>>,
        recorded_saves: RefCell<Vec<WorkspaceId>>,
        recorded_bootstrap_creates: RefCell<Vec<(WorkspaceId, WorkspaceMembershipId)>>,
        saved_workspaces: RefCell<Vec<Workspace>>,
        recorded_list_all: Cell<usize>,
        recorded_list_for_ids: RefCell<Vec<Vec<WorkspaceId>>>,
    }

    impl RecordingWorkspaceRepository {
        fn new(workspace: Workspace) -> Self {
            Self::with_workspaces(vec![workspace])
        }

        fn with_workspaces(workspaces: Vec<Workspace>) -> Self {
            Self {
                workspaces: RefCell::new(workspaces),
                recorded_gets: RefCell::new(Vec::new()),
                recorded_saves: RefCell::new(Vec::new()),
                recorded_bootstrap_creates: RefCell::new(Vec::new()),
                saved_workspaces: RefCell::new(Vec::new()),
                recorded_list_all: Cell::new(0),
                recorded_list_for_ids: RefCell::new(Vec::new()),
            }
        }
    }

    impl WorkspaceRepository for RecordingWorkspaceRepository {
        fn get(&self, workspace_id: &WorkspaceId) -> WorkspaceResult<Workspace> {
            self.recorded_gets.borrow_mut().push(workspace_id.clone());
            self.workspaces
                .borrow()
                .iter()
                .find(|workspace| workspace.id() == workspace_id)
                .cloned()
                .ok_or_else(|| WorkspaceError::WorkspaceNotFound {
                    workspace_id: workspace_id.clone(),
                })
        }

        fn list_all(&self) -> WorkspaceResult<Vec<Workspace>> {
            self.recorded_list_all.set(self.recorded_list_all.get() + 1);
            Ok(self.workspaces.borrow().clone())
        }

        fn list_for_ids(&self, workspace_ids: &[WorkspaceId]) -> WorkspaceResult<Vec<Workspace>> {
            self.recorded_list_for_ids
                .borrow_mut()
                .push(workspace_ids.to_vec());
            Ok(self
                .workspaces
                .borrow()
                .iter()
                .filter(|workspace| {
                    workspace_ids
                        .iter()
                        .any(|workspace_id| workspace.id() == workspace_id)
                })
                .cloned()
                .collect())
        }

        fn create_with_owner(
            &self,
            workspace: &Workspace,
            owner_membership: &WorkspaceMembership,
        ) -> WorkspaceResult<()> {
            self.recorded_bootstrap_creates
                .borrow_mut()
                .push((workspace.id().clone(), owner_membership.id().clone()));
            self.saved_workspaces.borrow_mut().push(workspace.clone());
            let mut workspaces = self.workspaces.borrow_mut();
            if let Some(index) = workspaces
                .iter()
                .position(|stored_workspace| stored_workspace.id() == workspace.id())
            {
                workspaces[index] = workspace.clone();
            } else {
                workspaces.push(workspace.clone());
            }
            Ok(())
        }

        fn save(&self, workspace: &Workspace) -> WorkspaceResult<()> {
            self.recorded_saves
                .borrow_mut()
                .push(workspace.id().clone());
            self.saved_workspaces.borrow_mut().push(workspace.clone());
            let mut workspaces = self.workspaces.borrow_mut();
            if let Some(index) = workspaces
                .iter()
                .position(|stored_workspace| stored_workspace.id() == workspace.id())
            {
                workspaces[index] = workspace.clone();
            } else {
                workspaces.push(workspace.clone());
            }
            Ok(())
        }
    }

    #[derive(Debug)]
    struct RecordingUserRepository {
        users: Vec<User>,
        recorded_gets: RefCell<Vec<UserId>>,
        recorded_list_by_ids: RefCell<Vec<Vec<UserId>>>,
    }

    impl RecordingUserRepository {
        fn new(user: User) -> Self {
            Self::with_users(vec![user])
        }

        fn with_users(users: Vec<User>) -> Self {
            Self {
                users,
                recorded_gets: RefCell::new(Vec::new()),
                recorded_list_by_ids: RefCell::new(Vec::new()),
            }
        }
    }

    impl UserRepository for RecordingUserRepository {
        fn get(&self, user_id: &UserId) -> WorkspaceResult<User> {
            self.recorded_gets.borrow_mut().push(user_id.clone());
            self.users
                .iter()
                .find(|user| user.id() == user_id)
                .cloned()
                .ok_or_else(|| WorkspaceError::UserNotFound {
                    user_id: user_id.clone(),
                })
        }

        fn list_by_ids(&self, user_ids: &[UserId]) -> WorkspaceResult<Vec<User>> {
            self.recorded_list_by_ids
                .borrow_mut()
                .push(user_ids.to_vec());
            Ok(self
                .users
                .iter()
                .filter(|user| user_ids.iter().any(|user_id| user.id() == user_id))
                .cloned()
                .collect())
        }
    }

    #[derive(Debug)]
    struct RecordingMembershipRepository {
        memberships: RefCell<Vec<WorkspaceMembership>>,
        recorded_find_for_workspace_user: RefCell<Vec<(WorkspaceId, UserId)>>,
        recorded_list_for_user: RefCell<Vec<UserId>>,
        recorded_list_for_workspace: RefCell<Vec<WorkspaceId>>,
        recorded_saves: RefCell<Vec<WorkspaceMembershipId>>,
        recorded_atomic_saves: RefCell<
            Vec<(
                WorkspaceMembershipId,
                WorkspaceId,
                crate::workspace::WorkspaceLastUpdated,
            )>,
        >,
        recorded_removes: RefCell<Vec<WorkspaceMembershipId>>,
        recorded_atomic_removes: RefCell<
            Vec<(
                WorkspaceId,
                WorkspaceMembershipId,
                crate::workspace::WorkspaceLastUpdated,
            )>,
        >,
        next_last_updated: RefCell<crate::workspace::WorkspaceLastUpdated>,
    }

    impl RecordingMembershipRepository {
        fn new(membership: WorkspaceMembership) -> Self {
            Self::with_memberships(vec![membership])
        }

        fn empty() -> Self {
            Self::with_memberships(Vec::new())
        }

        fn with_memberships(memberships: Vec<WorkspaceMembership>) -> Self {
            Self {
                memberships: RefCell::new(memberships),
                recorded_find_for_workspace_user: RefCell::new(Vec::new()),
                recorded_list_for_user: RefCell::new(Vec::new()),
                recorded_list_for_workspace: RefCell::new(Vec::new()),
                recorded_saves: RefCell::new(Vec::new()),
                recorded_atomic_saves: RefCell::new(Vec::new()),
                recorded_removes: RefCell::new(Vec::new()),
                recorded_atomic_removes: RefCell::new(Vec::new()),
                next_last_updated: RefCell::new(crate::workspace::WorkspaceLastUpdated::initial()),
            }
        }

        fn issue_last_updated(&self) -> crate::workspace::WorkspaceLastUpdated {
            let next = self.next_last_updated.borrow().clone().advance();
            *self.next_last_updated.borrow_mut() = next.clone();
            next
        }
    }

    impl MembershipRepository for RecordingMembershipRepository {
        fn get(
            &self,
            membership_id: &WorkspaceMembershipId,
        ) -> WorkspaceResult<WorkspaceMembership> {
            self.memberships
                .borrow()
                .iter()
                .find(|membership| membership.id() == membership_id)
                .cloned()
                .ok_or_else(|| WorkspaceError::MembershipNotFound {
                    membership_id: membership_id.clone(),
                })
        }

        fn find_for_workspace_user(
            &self,
            workspace_id: &WorkspaceId,
            user_id: &UserId,
        ) -> WorkspaceResult<WorkspaceMembership> {
            self.recorded_find_for_workspace_user
                .borrow_mut()
                .push((workspace_id.clone(), user_id.clone()));
            self.memberships
                .borrow()
                .iter()
                .find(|membership| {
                    membership.workspace_id() == workspace_id && membership.user_id() == user_id
                })
                .cloned()
                .ok_or_else(|| WorkspaceError::MembershipNotFoundForWorkspaceUser {
                    workspace_id: workspace_id.clone(),
                    user_id: user_id.clone(),
                })
        }

        fn list_for_user(&self, user_id: &UserId) -> WorkspaceResult<Vec<WorkspaceMembership>> {
            self.recorded_list_for_user
                .borrow_mut()
                .push(user_id.clone());
            Ok(self
                .memberships
                .borrow()
                .iter()
                .filter(|membership| membership.user_id() == user_id)
                .cloned()
                .collect())
        }

        fn list_for_workspace(
            &self,
            workspace_id: &WorkspaceId,
        ) -> WorkspaceResult<Vec<WorkspaceMembership>> {
            self.recorded_list_for_workspace
                .borrow_mut()
                .push(workspace_id.clone());
            Ok(self
                .memberships
                .borrow()
                .iter()
                .filter(|membership| membership.workspace_id() == workspace_id)
                .cloned()
                .collect())
        }

        fn remove(&self, membership_id: &WorkspaceMembershipId) -> WorkspaceResult<()> {
            self.recorded_removes
                .borrow_mut()
                .push(membership_id.clone());
            let mut memberships = self.memberships.borrow_mut();
            if let Some(index) = memberships
                .iter()
                .position(|membership| membership.id() == membership_id)
            {
                memberships.remove(index);
                Ok(())
            } else {
                Err(WorkspaceError::MembershipNotFound {
                    membership_id: membership_id.clone(),
                })
            }
        }

        fn save_with_workspace_bump(
            &self,
            membership: &WorkspaceMembership,
        ) -> WorkspaceResult<()> {
            let workspace_last_updated = self.issue_last_updated();
            self.recorded_atomic_saves.borrow_mut().push((
                membership.id().clone(),
                membership.workspace_id().clone(),
                workspace_last_updated,
            ));
            let mut memberships = self.memberships.borrow_mut();
            if let Some(index) = memberships
                .iter()
                .position(|stored_membership| stored_membership.id() == membership.id())
            {
                memberships[index] = membership.clone();
            } else {
                memberships.push(membership.clone());
            }
            Ok(())
        }

        fn remove_with_workspace_bump(
            &self,
            workspace_id: &WorkspaceId,
            membership_id: &WorkspaceMembershipId,
        ) -> WorkspaceResult<()> {
            let workspace_last_updated = self.issue_last_updated();
            self.recorded_atomic_removes.borrow_mut().push((
                workspace_id.clone(),
                membership_id.clone(),
                workspace_last_updated,
            ));
            let mut memberships = self.memberships.borrow_mut();
            if let Some(index) = memberships
                .iter()
                .position(|membership| membership.id() == membership_id)
            {
                memberships.remove(index);
                Ok(())
            } else {
                Err(WorkspaceError::MembershipNotFound {
                    membership_id: membership_id.clone(),
                })
            }
        }

        fn save(&self, membership: &WorkspaceMembership) -> WorkspaceResult<()> {
            self.recorded_saves
                .borrow_mut()
                .push(membership.id().clone());
            let mut memberships = self.memberships.borrow_mut();
            if let Some(index) = memberships
                .iter()
                .position(|stored_membership| stored_membership.id() == membership.id())
            {
                memberships[index] = membership.clone();
            } else {
                memberships.push(membership.clone());
            }
            Ok(())
        }
    }

    #[derive(Debug)]
    struct RecordingSecretStore {
        api_keys: Vec<WorkspaceApiKeyMetadata>,
        recorded_api_key_reads: RefCell<Vec<WorkspaceId>>,
    }

    impl RecordingSecretStore {
        fn new(api_keys: Vec<WorkspaceApiKeyMetadata>) -> Self {
            Self {
                api_keys,
                recorded_api_key_reads: RefCell::new(Vec::new()),
            }
        }
    }

    impl SecretStore for RecordingSecretStore {
        fn list_api_keys(
            &self,
            workspace_id: &WorkspaceId,
        ) -> WorkspaceResult<Vec<WorkspaceApiKeyMetadata>> {
            self.recorded_api_key_reads
                .borrow_mut()
                .push(workspace_id.clone());
            Ok(self.api_keys.clone())
        }
    }
}
