use super::{
    WorkspaceApiKeyId, WorkspaceLastUpdated, WorkspaceSecretRefId, WorkspaceSecretVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSecretRef {
    pub secret_ref_id: WorkspaceSecretRefId,
    pub version: WorkspaceSecretVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceCredentialStatus {
    Active,
    Rotated,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceApiKeyMetadata {
    pub api_key_id: WorkspaceApiKeyId,
    pub label: String,
    pub secret_ref: WorkspaceSecretRef,
    pub status: WorkspaceCredentialStatus,
    pub created_at: WorkspaceLastUpdated,
    pub rotated_at: Option<WorkspaceLastUpdated>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceApiKeySecret {
    pub api_key_id: WorkspaceApiKeyId,
    pub api_secret: String,
    pub status: WorkspaceCredentialStatus,
    pub version: WorkspaceSecretVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSigningProfile {
    pub active_secret_ref: WorkspaceSecretRef,
}
