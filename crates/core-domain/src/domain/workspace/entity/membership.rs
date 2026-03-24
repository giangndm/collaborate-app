use crate::workspace::{UserId, WorkspaceId, WorkspaceMembershipId, WorkspaceRole};

/// Exists to keep workspace participation separate from the global user
/// record in the domain skeleton.
///
/// This struct represents one user's membership in one workspace together with
/// the role granted inside that workspace.
///
/// Future developers should use this type whenever a use case needs
/// to answer who belongs to a workspace or what permissions that user has there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMembership {
    id: WorkspaceMembershipId,
    workspace_id: WorkspaceId,
    user_id: UserId,
    role: WorkspaceRole,
}

impl WorkspaceMembership {
    // TODO(task-4): Enforce membership creation invariants here once workspace
    // access rules are known, such as preventing duplicate active memberships
    // and validating that the assigned role is allowed for the invitation path.
    pub fn new(
        id: WorkspaceMembershipId,
        workspace_id: WorkspaceId,
        user_id: UserId,
        role: WorkspaceRole,
    ) -> Self {
        Self {
            id,
            workspace_id,
            user_id,
            role,
        }
    }

    pub fn id(&self) -> &WorkspaceMembershipId {
        &self.id
    }

    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user_id
    }

    pub fn role(&self) -> WorkspaceRole {
        self.role
    }

    pub fn is_owner(&self) -> bool {
        self.role == WorkspaceRole::Owner
    }

    // TODO(task-4): Add authorization and downgrade/upgrade safeguards here,
    // including checks that the last owner cannot be demoted without a
    // replacement and that role changes are audit logged.
    pub fn change_role(&mut self, role: WorkspaceRole) {
        self.role = role;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{UserId, WorkspaceId, WorkspaceMembershipId, WorkspaceRole};

    #[test]
    fn membership_construction_keeps_workspace_role_mapping_explicit() {
        let membership = WorkspaceMembership::new(
            WorkspaceMembershipId::new("membership_alpha"),
            WorkspaceId::new("ws_alpha"),
            UserId::new("user_alpha"),
            WorkspaceRole::Member,
        );

        assert_eq!(
            membership.id(),
            &WorkspaceMembershipId::new("membership_alpha")
        );
        assert_eq!(membership.workspace_id(), &WorkspaceId::new("ws_alpha"));
        assert_eq!(membership.user_id(), &UserId::new("user_alpha"));
        assert_eq!(membership.role(), WorkspaceRole::Member);
    }

    #[test]
    fn membership_role_change_replaces_stored_role() {
        let mut membership = WorkspaceMembership::new(
            WorkspaceMembershipId::new("membership_beta"),
            WorkspaceId::new("ws_beta"),
            UserId::new("user_beta"),
            WorkspaceRole::Member,
        );

        membership.change_role(WorkspaceRole::Owner);

        assert_eq!(membership.role(), WorkspaceRole::Owner);
    }
}
