//! Guard types convert verified actor or integration context into typed permissions.

mod integration_guard;
mod super_admin_guard;
mod workspace_creator_guard;
mod workspace_member_guard;

pub use integration_guard::*;
pub use super_admin_guard::*;
pub use workspace_creator_guard::*;
pub use workspace_member_guard::*;

#[cfg(test)]
mod tests {
    use super::{IntegrationGuard, SuperAdminGuard, WorkspaceCreatorGuard, WorkspaceMemberGuard};
    use crate::workspace::{
        DisplayName, GlobalUserRole, User, UserEmail, UserId, UserProfile, WorkspaceId,
        WorkspaceRole,
    };

    fn verified_member_guard(
        workspace_id: WorkspaceId,
        role: WorkspaceRole,
    ) -> WorkspaceMemberGuard {
        unsafe { WorkspaceMemberGuard::new_verified(workspace_id, role) }
    }

    #[test]
    fn workspace_creator_guard_can_be_constructed_for_bootstrap_flow() {
        let actor = User::new(
            UserId::new("user_super_admin_creator"),
            GlobalUserRole::SuperAdmin,
            UserProfile::new(
                UserEmail::new("creator@example.com"),
                DisplayName::new("Creator"),
            ),
        );
        let guard = WorkspaceCreatorGuard::try_from_actor(&actor)
            .expect("super admin role should mint creator guard");

        assert_eq!(guard.actor_user_id(), actor.id());
    }

    #[test]
    fn workspace_members_derive_baseline_permissions_from_workspace_role() {
        let owner_guard = verified_member_guard(WorkspaceId::new("ws_owner"), WorkspaceRole::Owner);
        let member_guard =
            verified_member_guard(WorkspaceId::new("ws_member"), WorkspaceRole::Member);

        assert_eq!(
            owner_guard.read_permission().workspace_id(),
            owner_guard.workspace_id()
        );
        assert_eq!(
            owner_guard.write_permission().unwrap().workspace_id(),
            owner_guard.workspace_id()
        );
        assert_eq!(
            member_guard.read_permission().workspace_id(),
            member_guard.workspace_id()
        );
        assert!(member_guard.write_permission().is_none());
    }

    #[test]
    fn super_admin_guard_derives_workspace_scoped_permissions_only() {
        let guard = SuperAdminGuard::try_from_role(GlobalUserRole::SuperAdmin)
            .expect("super admin role should produce a guard");
        let workspace_id = WorkspaceId::new("ws_any");

        assert_eq!(
            guard.read_permission(workspace_id.clone()).workspace_id(),
            &workspace_id
        );
        assert_eq!(
            guard.write_permission(workspace_id.clone()).workspace_id(),
            &workspace_id
        );
    }

    #[test]
    fn integration_guard_mints_cross_workspace_read_permission() {
        let guard = unsafe { IntegrationGuard::new_verified() };
        let another_guard = unsafe { IntegrationGuard::new_verified() };

        assert_eq!(
            guard.workspaces_read_permission(),
            another_guard.workspaces_read_permission()
        );
    }

    #[test]
    fn non_super_admin_role_does_not_produce_super_admin_guard() {
        assert!(SuperAdminGuard::try_from_role(GlobalUserRole::Member).is_none());
    }

    #[test]
    fn non_super_admin_role_does_not_produce_workspace_creator_guard() {
        let actor = User::new(
            UserId::new("user_member_creator"),
            GlobalUserRole::Member,
            UserProfile::new(
                UserEmail::new("member@example.com"),
                DisplayName::new("Member"),
            ),
        );

        assert!(WorkspaceCreatorGuard::try_from_actor(&actor).is_none());
    }

    #[test]
    fn suspended_super_admin_does_not_produce_workspace_creator_guard() {
        let mut actor = User::new(
            UserId::new("user_suspended_super_admin_creator"),
            GlobalUserRole::SuperAdmin,
            UserProfile::new(
                UserEmail::new("suspended@example.com"),
                DisplayName::new("Suspended Creator"),
            ),
        );
        actor.suspend();

        assert!(WorkspaceCreatorGuard::try_from_actor(&actor).is_none());
    }
}
