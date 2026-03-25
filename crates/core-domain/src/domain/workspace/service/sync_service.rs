use crate::workspace::{
    SecretStore, WorkspaceCredentialVerifier, WorkspaceId, WorkspaceReadPermission,
    WorkspaceRepository, WorkspaceResult, WorkspaceSyncPayload, WorkspacesReadPermission,
};

/// Builds sync-export payloads from typed workspace and secret-store contracts.
///
/// This concrete service exists because sync export is a first-class workspace
/// use case, not merely a repository detail. It composes the canonical
/// workspace aggregate with secret-backed API-key metadata into the narrow
/// payload that downstream sync consumers need.
///
/// Use this service when the caller already has workspace-scoped read access and
/// needs a `WorkspaceSyncPayload` for a gateway, control-plane bridge, or other
/// read model boundary. In this skeleton, the workspace aggregate is the
/// baseline authority for the signing profile, while the secret store only adds
/// API-key metadata.
#[derive(Debug)]
pub struct WorkspaceSyncService<WorkspaceRepo, SecretStorePort> {
    pub(crate) workspace_repository: WorkspaceRepo,
    pub(crate) secret_store: SecretStorePort,
}

impl<WorkspaceRepo, SecretStorePort> WorkspaceSyncService<WorkspaceRepo, SecretStorePort>
where
    WorkspaceRepo: WorkspaceRepository,
    SecretStorePort: SecretStore,
{
    /// Creates the service from the repository and secret-store contracts it orchestrates.
    pub fn new(workspace_repository: WorkspaceRepo, secret_store: SecretStorePort) -> Self {
        Self {
            workspace_repository,
            secret_store,
        }
    }

    /// Exports the workspace sync payload for one workspace-scoped read request.
    pub async fn export_sync_payload(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<WorkspaceSyncPayload> {
        self.export_sync_payload_internal(permission.workspace_id())
            .await
    }

    /// Exports the workspace sync payload for a machine-to-machine read request.
    pub async fn export_sync_payload_with_workspaces_read_permission(
        &self,
        _permission: &WorkspacesReadPermission,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<WorkspaceSyncPayload> {
        self.export_sync_payload_internal(workspace_id).await
    }

    async fn export_sync_payload_internal(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<WorkspaceSyncPayload> {
        let workspace = self.workspace_repository.get(workspace_id).await?;

        // TODO(task-7): Keep this payload intentionally small. Extend it only
        // after the gateway contract proves it needs more fields or stronger
        // versioning guarantees.
        Ok(WorkspaceSyncPayload {
            workspace_id: workspace.id().clone(),
            status: workspace.status(),
            last_updated: workspace.last_updated(),
            policy: workspace.policy().clone(),
            default_room_policy: workspace.default_room_policy().clone(),
            credential_verifiers: self
                .secret_store
                .list_api_keys(workspace_id)
                .await?
                .iter()
                .map(WorkspaceCredentialVerifier::from_metadata)
                .collect(),
        })
    }
}
