use super::{WorkspaceApiKeyId, WorkspaceSecretRefId, WorkspaceSecretVersion};

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
    pub secret_ref: WorkspaceSecretRef,
    pub status: WorkspaceCredentialStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSigningProfile {
    pub active_secret_ref: WorkspaceSecretRef,
}
