use super::{
    WorkspaceApiKeyMetadata, WorkspaceId, WorkspacePolicy, WorkspaceSigningProfile, WorkspaceStatus,
};

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
    pub policy: WorkspacePolicy,
    pub signing_profile: Option<WorkspaceSigningProfile>,
    pub api_keys: Vec<WorkspaceApiKeyMetadata>,
}
