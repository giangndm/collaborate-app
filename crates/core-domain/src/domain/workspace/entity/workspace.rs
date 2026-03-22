use crate::workspace::{WorkspaceId, WorkspacePolicy, WorkspaceSigningProfile, WorkspaceStatus};

/// Exists to keep workspace-wide state in one typed domain entity for the
/// skeleton.
///
/// This struct represents the canonical record for a single workspace, covering
/// its identity, lifecycle status, policy, and active signing profile.
///
/// Future developers should use this type whenever a rule or use case needs
/// to read or update workspace-level state instead of spreading that state
/// across ad hoc values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    id: WorkspaceId,
    status: WorkspaceStatus,
    policy: WorkspacePolicy,
    signing_profile: WorkspaceSigningProfile,
}

impl Workspace {
    // TODO(task-4): Validate creation invariants here once the aggregate rules
    // are finalized, such as which default policy values are allowed and
    // whether every new workspace must start with a provisioned signing secret.
    pub fn new(
        id: WorkspaceId,
        policy: WorkspacePolicy,
        signing_profile: WorkspaceSigningProfile,
    ) -> Self {
        Self {
            id,
            status: WorkspaceStatus::Active,
            policy,
            signing_profile,
        }
    }

    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn status(&self) -> WorkspaceStatus {
        self.status
    }

    pub fn policy(&self) -> &WorkspacePolicy {
        &self.policy
    }

    pub fn signing_profile(&self) -> &WorkspaceSigningProfile {
        &self.signing_profile
    }

    // TODO(task-4): Enforce lifecycle transition rules once business behavior
    // is implemented, for example blocking activation from disabled state
    // without an explicit recovery workflow.
    pub fn activate(&mut self) {
        self.status = WorkspaceStatus::Active;
    }

    // TODO(task-4): Add suspension preconditions and side effects here, such as
    // recording the suspension reason and coordinating any access revocation.
    pub fn suspend(&mut self) {
        self.status = WorkspaceStatus::Suspended;
    }

    // TODO(task-4): Replace this direct assignment with the permanent disable
    // policy once defined, including checks for irreversible shutdown rules and
    // any cleanup obligations.
    pub fn disable(&mut self) {
        self.status = WorkspaceStatus::Disabled;
    }

    // TODO(task-4): Verify rotation rules before swapping profiles, including
    // secret ownership, version monotonicity, and any grace period for active
    // signatures that still reference the previous key material.
    pub fn rotate_signing_profile(&mut self, signing_profile: WorkspaceSigningProfile) {
        self.signing_profile = signing_profile;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{
        GuestAccessPolicy, WorkspaceId, WorkspacePolicy, WorkspaceSecretRef, WorkspaceSecretRefId,
        WorkspaceSecretVersion, WorkspaceSigningProfile, WorkspaceStatus,
    };

    #[test]
    fn workspace_construction_keeps_minimal_typed_state() {
        let workspace = Workspace::new(
            WorkspaceId::new("ws_alpha"),
            WorkspacePolicy {
                guest_access: GuestAccessPolicy::Allowed,
            },
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_alpha"),
                    version: WorkspaceSecretVersion::new(1),
                },
            },
        );

        assert_eq!(workspace.id(), &WorkspaceId::new("ws_alpha"));
        assert_eq!(workspace.status(), WorkspaceStatus::Active);
        assert_eq!(workspace.policy().guest_access, GuestAccessPolicy::Allowed);
        assert_eq!(
            workspace.signing_profile().active_secret_ref.secret_ref_id,
            WorkspaceSecretRefId::new("secret_alpha")
        );
    }

    #[test]
    fn workspace_status_transitions_preserve_simple_invariants() {
        let mut workspace = Workspace::new(
            WorkspaceId::new("ws_beta"),
            WorkspacePolicy::default(),
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_beta"),
                    version: WorkspaceSecretVersion::new(2),
                },
            },
        );

        workspace.suspend();
        assert_eq!(workspace.status(), WorkspaceStatus::Suspended);

        workspace.disable();
        assert_eq!(workspace.status(), WorkspaceStatus::Disabled);

        workspace.activate();
        assert_eq!(workspace.status(), WorkspaceStatus::Active);

        workspace.rotate_signing_profile(WorkspaceSigningProfile {
            active_secret_ref: WorkspaceSecretRef {
                secret_ref_id: WorkspaceSecretRefId::new("secret_beta_rotated"),
                version: WorkspaceSecretVersion::new(3),
            },
        });

        assert_eq!(
            workspace.signing_profile().active_secret_ref.secret_ref_id,
            WorkspaceSecretRefId::new("secret_beta_rotated")
        );
    }
}
