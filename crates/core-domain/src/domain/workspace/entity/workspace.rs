use crate::workspace::{
    DefaultRoomPolicy, WorkspaceId, WorkspaceLastUpdated, WorkspaceName, WorkspacePolicy,
    WorkspaceSigningProfile, WorkspaceSlug, WorkspaceStatus, WorkspaceUpdate,
};

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
    name: WorkspaceName,
    slug: WorkspaceSlug,
    status: WorkspaceStatus,
    policy: WorkspacePolicy,
    default_room_policy: DefaultRoomPolicy,
    last_updated: WorkspaceLastUpdated,
    signing_profile: WorkspaceSigningProfile,
}

impl Workspace {
    // TODO(task-4): Validate creation invariants here once the aggregate rules
    // are finalized, such as which default policy values are allowed and
    // whether every new workspace must start with a provisioned signing secret.
    pub fn new(
        id: WorkspaceId,
        name: WorkspaceName,
        slug: WorkspaceSlug,
        status: WorkspaceStatus,
        policy: WorkspacePolicy,
        default_room_policy: DefaultRoomPolicy,
        signing_profile: WorkspaceSigningProfile,
    ) -> Self {
        Self {
            id,
            name,
            slug,
            status,
            policy,
            default_room_policy,
            last_updated: WorkspaceLastUpdated::now(),
            signing_profile,
        }
    }

    pub fn rehydrate(
        id: WorkspaceId,
        name: WorkspaceName,
        slug: WorkspaceSlug,
        status: WorkspaceStatus,
        policy: WorkspacePolicy,
        default_room_policy: DefaultRoomPolicy,
        last_updated: WorkspaceLastUpdated,
        signing_profile: WorkspaceSigningProfile,
    ) -> Self {
        Self {
            id,
            name,
            slug,
            status,
            policy,
            default_room_policy,
            last_updated,
            signing_profile,
        }
    }

    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn status(&self) -> WorkspaceStatus {
        self.status
    }

    pub fn name(&self) -> &WorkspaceName {
        &self.name
    }

    pub fn slug(&self) -> &WorkspaceSlug {
        &self.slug
    }

    pub fn policy(&self) -> &WorkspacePolicy {
        &self.policy
    }

    pub fn default_room_policy(&self) -> &DefaultRoomPolicy {
        &self.default_room_policy
    }

    pub fn last_updated(&self) -> WorkspaceLastUpdated {
        self.last_updated.clone()
    }

    pub fn signing_profile(&self) -> &WorkspaceSigningProfile {
        &self.signing_profile
    }

    // TODO(task-4): Enforce lifecycle transition rules once business behavior
    // is implemented, for example blocking activation from disabled state
    // without an explicit recovery workflow.
    pub fn activate(&mut self) {
        self.status = WorkspaceStatus::Active;
        self.bump_last_updated();
    }

    // TODO(task-4): Add suspension preconditions and side effects here, such as
    // recording the suspension reason and coordinating any access revocation.
    pub fn suspend(&mut self) {
        self.status = WorkspaceStatus::Suspended;
        self.bump_last_updated();
    }

    // TODO(task-4): Replace this direct assignment with the permanent disable
    // policy once defined, including checks for irreversible shutdown rules and
    // any cleanup obligations.
    pub fn disable(&mut self) {
        self.status = WorkspaceStatus::Disabled;
        self.bump_last_updated();
    }

    // TODO(task-4): Verify rotation rules before swapping profiles, including
    // secret ownership, version monotonicity, and any grace period for active
    // signatures that still reference the previous key material.
    pub fn rotate_signing_profile(&mut self, signing_profile: WorkspaceSigningProfile) {
        self.signing_profile = signing_profile;
        self.bump_last_updated();
    }

    pub fn update_default_room_policy(&mut self, default_room_policy: DefaultRoomPolicy) {
        self.default_room_policy = default_room_policy;
        self.bump_last_updated();
    }

    pub fn apply_update(&mut self, update: WorkspaceUpdate) {
        self.name = update.name;
        self.status = update.status;
        self.default_room_policy = update.default_room_policy;
        self.bump_last_updated();
    }

    pub fn bump_last_updated(&mut self) {
        self.last_updated = self.last_updated.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{
        DefaultRoomPolicy, GuestAccessPolicy, WorkspaceId, WorkspaceLastUpdated, WorkspaceName,
        WorkspacePolicy, WorkspaceSecretRef, WorkspaceSecretRefId, WorkspaceSecretVersion,
        WorkspaceSigningProfile, WorkspaceSlug, WorkspaceStatus, WorkspaceUpdate,
    };

    #[test]
    fn workspace_construction_keeps_minimal_typed_state() {
        let workspace = Workspace::new(
            WorkspaceId::new("ws_alpha"),
            WorkspaceName::new("Workspace Alpha"),
            WorkspaceSlug::new("workspace-alpha"),
            WorkspaceStatus::Suspended,
            WorkspacePolicy {
                guest_access: GuestAccessPolicy::Allowed,
            },
            DefaultRoomPolicy::new(false, 3600),
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_alpha"),
                    version: WorkspaceSecretVersion::new(1),
                },
            },
        );

        assert_eq!(workspace.id(), &WorkspaceId::new("ws_alpha"));
        assert_eq!(workspace.name(), &WorkspaceName::new("Workspace Alpha"));
        assert_eq!(workspace.slug(), &WorkspaceSlug::new("workspace-alpha"));
        assert_eq!(workspace.status(), WorkspaceStatus::Suspended);
        assert_eq!(workspace.policy().guest_access, GuestAccessPolicy::Allowed);
        assert_eq!(
            workspace.default_room_policy(),
            &DefaultRoomPolicy::new(false, 3600)
        );
        assert!(workspace.last_updated() > WorkspaceLastUpdated::initial());
        assert_eq!(
            workspace.signing_profile().active_secret_ref.secret_ref_id,
            WorkspaceSecretRefId::new("secret_alpha")
        );
    }

    #[test]
    fn workspace_status_transitions_preserve_simple_invariants() {
        let mut workspace = Workspace::new(
            WorkspaceId::new("ws_beta"),
            WorkspaceName::new("Workspace Beta"),
            WorkspaceSlug::new("workspace-beta"),
            WorkspaceStatus::Active,
            WorkspacePolicy::default(),
            DefaultRoomPolicy::new(false, 3600),
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_beta"),
                    version: WorkspaceSecretVersion::new(2),
                },
            },
        );

        let created_at = workspace.last_updated();

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
        assert!(workspace.last_updated() > created_at);
    }

    #[test]
    fn workspace_update_replaces_metadata_status_and_policy() {
        let mut workspace = Workspace::new(
            WorkspaceId::new("ws_gamma"),
            WorkspaceName::new("Workspace Gamma"),
            WorkspaceSlug::new("workspace-gamma"),
            WorkspaceStatus::Active,
            WorkspacePolicy::default(),
            DefaultRoomPolicy::new(false, 3600),
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_gamma"),
                    version: WorkspaceSecretVersion::new(7),
                },
            },
        );

        workspace.apply_update(WorkspaceUpdate::new(
            WorkspaceName::new("Workspace Gamma Updated"),
            WorkspaceStatus::Suspended,
            DefaultRoomPolicy::new(true, 1800),
        ));

        assert_eq!(
            workspace.name(),
            &WorkspaceName::new("Workspace Gamma Updated")
        );
        assert_eq!(workspace.slug(), &WorkspaceSlug::new("workspace-gamma"));
        assert_eq!(workspace.status(), WorkspaceStatus::Suspended);
        assert_eq!(workspace.policy().guest_access, GuestAccessPolicy::Denied);
        assert_eq!(
            workspace.default_room_policy(),
            &DefaultRoomPolicy::new(true, 1800)
        );
    }

    #[test]
    fn workspace_rehydrate_preserves_persisted_last_updated() {
        let last_updated = WorkspaceLastUpdated::from_rfc3339("2026-03-23T10:00:00Z")
            .expect("rfc3339 timestamp should parse");

        let workspace = Workspace::rehydrate(
            WorkspaceId::new("ws_delta"),
            WorkspaceName::new("Workspace Delta"),
            WorkspaceSlug::new("workspace-delta"),
            WorkspaceStatus::Active,
            WorkspacePolicy::default(),
            DefaultRoomPolicy::new(false, 3600),
            last_updated.clone(),
            WorkspaceSigningProfile {
                active_secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new("secret_delta"),
                    version: WorkspaceSecretVersion::new(8),
                },
            },
        );

        assert_eq!(workspace.last_updated(), last_updated);
    }
}
