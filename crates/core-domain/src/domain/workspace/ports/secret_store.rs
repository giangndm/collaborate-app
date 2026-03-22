use crate::workspace::{WorkspaceApiKeyMetadata, WorkspaceId, WorkspaceResult};

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
}
