use crate::workspace::{
    WorkspaceApiKeyId, WorkspaceApiKeyMetadata, WorkspaceApiKeySecret, WorkspaceId, WorkspaceResult,
};

/// Abstracts sensitive workspace credential metadata behind an intent-level
/// boundary.
///
/// Use this port when a workspace use case needs API-key metadata tied to
/// secrets, but should remain isolated from infrastructure details such as
/// vault vendors, encryption schemes, or secret transport.
pub trait SecretStore {
    /// Returns API-key metadata in storage-defined order; callers must not rely on sorting.
    fn list_api_keys(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<Vec<WorkspaceApiKeyMetadata>>;

    /// Creates a new API key and returns the stored metadata with plaintext secret once.
    fn create_api_key(
        &self,
        workspace_id: &WorkspaceId,
        label: &str,
    ) -> WorkspaceResult<WorkspaceApiKeySecret>;

    /// Rotates an API key secret and returns updated metadata with the new plaintext once.
    fn rotate_api_key_secret(
        &self,
        workspace_id: &WorkspaceId,
        api_key_id: &WorkspaceApiKeyId,
    ) -> WorkspaceResult<WorkspaceApiKeySecret>;
}
