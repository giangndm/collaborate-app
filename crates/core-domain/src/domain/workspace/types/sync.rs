use crate::workspace::{User, Workspace};
use chrono::{DateTime, SecondsFormat, Utc};

use super::{
    DisplayName, GlobalUserRole, UserEmail, UserId, UserStatus, WorkspaceApiKeyMetadata,
    WorkspaceCredentialStatus, WorkspaceId, WorkspaceMembershipId, WorkspacePolicy, WorkspaceRole,
    WorkspaceSecretRefId, WorkspaceSecretVersion, WorkspaceStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceName(String);

impl WorkspaceName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceSlug(String);

impl WorkspaceSlug {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceLastUpdated(DateTime<Utc>);

impl WorkspaceLastUpdated {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn initial() -> Self {
        Self(DateTime::from_timestamp(0, 0).expect("unix epoch should be valid"))
    }

    pub fn from_rfc3339(value: &str) -> Result<Self, chrono::ParseError> {
        Ok(Self(
            DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc),
        ))
    }

    pub fn to_rfc3339(&self) -> String {
        self.0.to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    pub fn advance(&self) -> Self {
        let now = Utc::now();
        let minimum_next = self.0 + chrono::TimeDelta::milliseconds(1);

        if now > minimum_next {
            Self(now)
        } else {
            Self(minimum_next)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultRoomPolicy {
    pub guest_join_enabled: bool,
    pub token_ttl_seconds: u32,
}

impl DefaultRoomPolicy {
    pub fn new(guest_join_enabled: bool, token_ttl_seconds: u32) -> Self {
        Self {
            guest_join_enabled,
            token_ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSummary {
    pub workspace_id: WorkspaceId,
    pub name: WorkspaceName,
    pub slug: WorkspaceSlug,
    pub status: WorkspaceStatus,
    pub last_updated: WorkspaceLastUpdated,
}

impl WorkspaceSummary {
    pub fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            workspace_id: workspace.id().clone(),
            name: workspace.name().clone(),
            slug: workspace.slug().clone(),
            status: workspace.status(),
            last_updated: workspace.last_updated(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDetail {
    pub workspace_id: WorkspaceId,
    pub name: WorkspaceName,
    pub slug: WorkspaceSlug,
    pub status: WorkspaceStatus,
    pub policy: WorkspacePolicy,
    pub default_room_policy: DefaultRoomPolicy,
    pub last_updated: WorkspaceLastUpdated,
}

impl WorkspaceDetail {
    pub fn from_workspace(workspace: &Workspace) -> Self {
        Self {
            workspace_id: workspace.id().clone(),
            name: workspace.name().clone(),
            slug: workspace.slug().clone(),
            status: workspace.status(),
            policy: workspace.policy().clone(),
            default_room_policy: workspace.default_room_policy().clone(),
            last_updated: workspace.last_updated(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceUpdate {
    pub name: WorkspaceName,
    pub status: WorkspaceStatus,
    pub default_room_policy: DefaultRoomPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceCredentialVerifier {
    pub api_key_id: super::WorkspaceApiKeyId,
    pub secret_ref_id: WorkspaceSecretRefId,
    pub version: WorkspaceSecretVersion,
    pub status: WorkspaceCredentialStatus,
}

impl WorkspaceCredentialVerifier {
    pub fn from_metadata(metadata: &WorkspaceApiKeyMetadata) -> Self {
        Self {
            api_key_id: metadata.api_key_id.clone(),
            secret_ref_id: metadata.secret_ref.secret_ref_id.clone(),
            version: metadata.secret_ref.version,
            status: metadata.status,
        }
    }
}

impl WorkspaceUpdate {
    pub fn new(
        name: WorkspaceName,
        status: WorkspaceStatus,
        default_room_policy: DefaultRoomPolicy,
    ) -> Self {
        Self {
            name,
            status,
            default_room_policy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMemberView {
    pub membership_id: WorkspaceMembershipId,
    pub workspace_id: WorkspaceId,
    pub user_id: UserId,
    pub workspace_role: WorkspaceRole,
    pub global_role: GlobalUserRole,
    pub user_status: UserStatus,
    pub email: UserEmail,
    pub display_name: DisplayName,
}

impl WorkspaceMemberView {
    pub fn new(
        membership_id: WorkspaceMembershipId,
        workspace_id: WorkspaceId,
        user: &User,
        workspace_role: WorkspaceRole,
    ) -> Self {
        Self {
            membership_id,
            workspace_id,
            user_id: user.id().clone(),
            workspace_role,
            global_role: user.role(),
            user_status: user.status(),
            email: user.profile().email().clone(),
            display_name: user.profile().display_name().clone(),
        }
    }

    pub fn from_parts(
        user: &User,
        workspace_role: WorkspaceRole,
        workspace_id: WorkspaceId,
    ) -> Self {
        Self::new(
            WorkspaceMembershipId::new(format!("{}:{}", workspace_id.as_str(), user.id().as_str())),
            workspace_id,
            user,
            workspace_role,
        )
    }
}

/// Snapshot exported from the workspace domain for sync-oriented boundaries.
///
/// This type exists so the domain can hand gateway or control-plane callers one
/// stable payload that combines the canonical workspace state with the
/// secret-backed metadata needed for downstream synchronization, without
/// exposing repositories or secret-store orchestration details.
///
/// Use `WorkspaceSyncPayload` when a caller already has workspace-scoped read
/// permission and needs a serializable view of one workspace for replication,
/// projection, or bridge-style integration work. Prefer the workspace aggregate
/// itself for in-domain read and write workflows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSyncPayload {
    pub workspace_id: WorkspaceId,
    pub status: WorkspaceStatus,
    pub last_updated: WorkspaceLastUpdated,
    pub policy: WorkspacePolicy,
    pub default_room_policy: DefaultRoomPolicy,
    pub credential_verifiers: Vec<WorkspaceCredentialVerifier>,
}
