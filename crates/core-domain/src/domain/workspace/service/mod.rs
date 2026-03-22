//! Workspace-domain orchestration services live here.

mod sync_service;
mod workspace_service;

pub use sync_service::*;
pub use workspace_service::*;

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::{WorkspaceService, WorkspaceSyncService};
    use crate::workspace::{
        DisplayName, GlobalUserRole, GuestAccessPolicy, MembershipRepository, SecretStore, User,
        UserEmail, UserId, UserProfile, UserRepository, Workspace, WorkspaceApiKeyId,
        WorkspaceApiKeyMetadata, WorkspaceCreatorGuard, WorkspaceCredentialStatus, WorkspaceError,
        WorkspaceId, WorkspaceMemberGuard, WorkspaceMembership, WorkspaceMembershipId,
        WorkspacePolicy, WorkspaceRepository, WorkspaceResult, WorkspaceRole, WorkspaceSecretRef,
        WorkspaceSecretRefId, WorkspaceSecretVersion, WorkspaceSigningProfile, WorkspaceStatus,
    };

    #[test]
    fn workspace_service_surface_stays_concrete_and_permission_driven() {
        let source = include_str!("workspace_service.rs");

        assert!(source.contains("pub struct WorkspaceService"));
        assert!(!source.contains("trait WorkspaceService"));
        assert!(!source.contains("CreateWorkspacePermission"));
        assert!(source.contains("guard: &WorkspaceCreatorGuard"));
        assert!(source.contains("permission: &WorkspaceReadPermission"));
        assert!(source.contains("permission: &WorkspaceWritePermission"));
    }

    #[test]
    fn workspace_sync_service_surface_stays_concrete_and_sync_focused() {
        let source = include_str!("sync_service.rs");

        assert!(source.contains("pub struct WorkspaceSyncService"));
        assert!(!source.contains("trait WorkspaceSyncService"));
        assert!(source.contains("WorkspaceSyncPayload"));
        assert!(source.contains("SecretStore"));
        assert!(source.contains("WorkspaceRepository"));
        assert!(!source.contains("get_signing_profile"));
    }

    #[test]
    fn workspace_service_orchestrates_basic_workspace_reads_and_writes() {
        let workspace = sample_workspace("ws_service");
        let membership =
            sample_membership("membership_service", workspace.id().clone(), "user_service");
        let user = sample_user("user_service");
        let workspace_repository = RecordingWorkspaceRepository::new(workspace.clone());
        let membership_repository = RecordingMembershipRepository::new(membership.clone());
        let user_repository = RecordingUserRepository::new(user.clone());
        let service =
            WorkspaceService::new(workspace_repository, user_repository, membership_repository);
        let creator_guard = WorkspaceCreatorGuard::new();
        let read_permission =
            WorkspaceMemberGuard::new(workspace.id().clone(), WorkspaceRole::Admin)
                .read_permission();
        let write_permission =
            WorkspaceMemberGuard::new(workspace.id().clone(), WorkspaceRole::Owner)
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
        assert_eq!(listed_memberships, vec![membership.clone()]);
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
                .workspace_repository
                .recorded_saves
                .borrow()
                .as_slice(),
            &[workspace.id().clone(), workspace.id().clone()]
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
            &[membership.id().clone()]
        );
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
        let permission = WorkspaceMemberGuard::new(workspace.id().clone(), WorkspaceRole::Member)
            .read_permission();

        let payload = service
            .export_sync_payload(&permission)
            .expect("sync payload export should compose repository and secret-store state");

        assert_eq!(payload.workspace_id, workspace.id().clone());
        assert_eq!(payload.status, WorkspaceStatus::Active);
        assert_eq!(payload.policy.guest_access, GuestAccessPolicy::Allowed);
        assert_eq!(
            payload.signing_profile,
            Some(workspace.signing_profile().clone())
        );
        assert_eq!(
            payload.api_keys,
            vec![sample_api_key("api_sync", "secret_api_sync", 9)]
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
            WorkspaceMemberGuard::new(WorkspaceId::new("ws_other"), WorkspaceRole::Owner)
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
        assert!(
            service
                .workspace_repository
                .recorded_saves
                .borrow()
                .is_empty()
        );
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
        let wrong_permission = WorkspaceMemberGuard::new(
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
        assert!(
            service
                .membership_repository
                .recorded_saves
                .borrow()
                .is_empty()
        );
    }

    fn sample_workspace(value: &str) -> Workspace {
        Workspace::new(
            WorkspaceId::new(value),
            WorkspacePolicy {
                guest_access: GuestAccessPolicy::Allowed,
            },
            sample_signing_profile("workspace_signing", 3),
        )
    }

    fn sample_user(value: &str) -> User {
        User::new(
            UserId::new(value),
            GlobalUserRole::Member,
            UserProfile::new(
                UserEmail::new(format!("{value}@example.com")),
                DisplayName::new(format!("User {value}")),
            ),
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
            WorkspaceRole::Admin,
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
        workspace: Workspace,
        recorded_gets: RefCell<Vec<WorkspaceId>>,
        recorded_saves: RefCell<Vec<WorkspaceId>>,
    }

    impl RecordingWorkspaceRepository {
        fn new(workspace: Workspace) -> Self {
            Self {
                workspace,
                recorded_gets: RefCell::new(Vec::new()),
                recorded_saves: RefCell::new(Vec::new()),
            }
        }
    }

    impl WorkspaceRepository for RecordingWorkspaceRepository {
        fn get(&self, workspace_id: &WorkspaceId) -> WorkspaceResult<Workspace> {
            self.recorded_gets.borrow_mut().push(workspace_id.clone());
            Ok(self.workspace.clone())
        }

        fn save(&self, workspace: &Workspace) -> WorkspaceResult<()> {
            self.recorded_saves
                .borrow_mut()
                .push(workspace.id().clone());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct RecordingUserRepository {
        user: User,
        recorded_gets: RefCell<Vec<UserId>>,
    }

    impl RecordingUserRepository {
        fn new(user: User) -> Self {
            Self {
                user,
                recorded_gets: RefCell::new(Vec::new()),
            }
        }
    }

    impl UserRepository for RecordingUserRepository {
        fn get(&self, user_id: &UserId) -> WorkspaceResult<User> {
            self.recorded_gets.borrow_mut().push(user_id.clone());
            Ok(self.user.clone())
        }
    }

    #[derive(Debug)]
    struct RecordingMembershipRepository {
        membership: WorkspaceMembership,
        recorded_find_for_workspace_user: RefCell<Vec<(WorkspaceId, UserId)>>,
        recorded_list_for_workspace: RefCell<Vec<WorkspaceId>>,
        recorded_saves: RefCell<Vec<WorkspaceMembershipId>>,
    }

    impl RecordingMembershipRepository {
        fn new(membership: WorkspaceMembership) -> Self {
            Self {
                membership,
                recorded_find_for_workspace_user: RefCell::new(Vec::new()),
                recorded_list_for_workspace: RefCell::new(Vec::new()),
                recorded_saves: RefCell::new(Vec::new()),
            }
        }
    }

    impl MembershipRepository for RecordingMembershipRepository {
        fn get(
            &self,
            membership_id: &WorkspaceMembershipId,
        ) -> WorkspaceResult<WorkspaceMembership> {
            if membership_id == self.membership.id() {
                Ok(self.membership.clone())
            } else {
                Err(WorkspaceError::MembershipNotFound {
                    membership_id: membership_id.clone(),
                })
            }
        }

        fn find_for_workspace_user(
            &self,
            workspace_id: &WorkspaceId,
            user_id: &UserId,
        ) -> WorkspaceResult<WorkspaceMembership> {
            self.recorded_find_for_workspace_user
                .borrow_mut()
                .push((workspace_id.clone(), user_id.clone()));
            Ok(self.membership.clone())
        }

        fn list_for_workspace(
            &self,
            workspace_id: &WorkspaceId,
        ) -> WorkspaceResult<Vec<WorkspaceMembership>> {
            self.recorded_list_for_workspace
                .borrow_mut()
                .push(workspace_id.clone());
            Ok(vec![self.membership.clone()])
        }

        fn save(&self, membership: &WorkspaceMembership) -> WorkspaceResult<()> {
            self.recorded_saves
                .borrow_mut()
                .push(membership.id().clone());
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
