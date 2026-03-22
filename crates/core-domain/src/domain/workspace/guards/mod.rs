//! Guard types convert verified actor context into typed permissions.

mod super_admin_guard;
mod workspace_creator_guard;
mod workspace_member_guard;

pub use super_admin_guard::*;
pub use workspace_creator_guard::*;
pub use workspace_member_guard::*;

#[cfg(test)]
mod tests {
    use super::{SuperAdminGuard, WorkspaceCreatorGuard, WorkspaceMemberGuard};
    use crate::workspace::{GlobalUserRole, WorkspaceId, WorkspaceRole};

    #[test]
    fn workspace_creator_guard_can_be_constructed_for_bootstrap_flow() {
        let guard = WorkspaceCreatorGuard::new();

        assert_eq!(guard, WorkspaceCreatorGuard::new());
    }

    #[test]
    fn workspace_members_derive_baseline_permissions_from_workspace_role() {
        let owner_guard =
            WorkspaceMemberGuard::new(WorkspaceId::new("ws_owner"), WorkspaceRole::Owner);
        let admin_guard =
            WorkspaceMemberGuard::new(WorkspaceId::new("ws_admin"), WorkspaceRole::Admin);
        let member_guard =
            WorkspaceMemberGuard::new(WorkspaceId::new("ws_member"), WorkspaceRole::Member);

        assert_eq!(
            owner_guard.read_permission().workspace_id(),
            owner_guard.workspace_id()
        );
        assert_eq!(
            owner_guard.write_permission().unwrap().workspace_id(),
            owner_guard.workspace_id()
        );
        assert_eq!(
            admin_guard.read_permission().workspace_id(),
            admin_guard.workspace_id()
        );
        assert_eq!(
            admin_guard.write_permission().unwrap().workspace_id(),
            admin_guard.workspace_id()
        );
        assert_eq!(
            member_guard.read_permission().workspace_id(),
            member_guard.workspace_id()
        );
        assert!(member_guard.write_permission().is_none());
    }

    #[test]
    fn super_admin_guard_derives_any_workspace_permission() {
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
    fn non_super_admin_role_does_not_produce_super_admin_guard() {
        assert!(SuperAdminGuard::try_from_role(GlobalUserRole::Member).is_none());
    }
}
