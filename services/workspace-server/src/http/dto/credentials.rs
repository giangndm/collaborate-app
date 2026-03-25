use core_domain::workspace::{WorkspaceApiKeyMetadata, WorkspaceApiKeySecret};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCredentialRequest {
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialMetadataDto {
    pub api_key_id: String,
    pub label: String,
    pub status: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

impl From<WorkspaceApiKeyMetadata> for CredentialMetadataDto {
    fn from(metadata: WorkspaceApiKeyMetadata) -> Self {
        Self {
            api_key_id: metadata.api_key_id.to_string(),
            label: metadata.label,
            status: metadata.status.to_string(),
            created_at: "".to_string(), // TODO: Add timing to domain model if needed
            last_used_at: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialSecretDto {
    pub api_key_id: String,
    pub api_secret: String,
}

impl From<WorkspaceApiKeySecret> for CredentialSecretDto {
    fn from(secret: WorkspaceApiKeySecret) -> Self {
        Self {
            api_key_id: secret.api_key_id.to_string(),
            api_secret: secret.api_secret,
        }
    }
}
