use serde::{Deserialize, Serialize};
use core_domain::workspace::WorkspaceSyncPayload;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceSyncDto {
    pub workspace_id: String,
    pub status: String,
    pub last_updated: String,
    pub credential_verifiers: Vec<CredentialVerifierDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CredentialVerifierDto {
    pub api_key_id: String,
    pub status: String,
}

impl From<WorkspaceSyncPayload> for WorkspaceSyncDto {
    fn from(payload: WorkspaceSyncPayload) -> Self {
        Self {
            workspace_id: payload.workspace_id.to_string(),
            status: payload.status.to_string(),
            last_updated: payload.last_updated.to_rfc3339(),
            credential_verifiers: payload
                .credential_verifiers
                .into_iter()
                .map(|v| CredentialVerifierDto {
                    api_key_id: v.api_key_id.to_string(),
                    status: v.status.to_string(),
                })
                .collect(),
        }
    }
}
