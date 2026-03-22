use crate::workspace::{
    SecretStore, WorkspaceReadPermission, WorkspaceRepository, WorkspaceResult,
    WorkspaceSyncPayload,
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
    pub fn export_sync_payload(
        &self,
        permission: &WorkspaceReadPermission,
    ) -> WorkspaceResult<WorkspaceSyncPayload> {
        let workspace = self.workspace_repository.get(permission.workspace_id())?;

        // TODO(task-7): Keep this payload intentionally small. Extend it only
        // after the gateway contract proves it needs more fields or stronger
        // versioning guarantees.
        Ok(WorkspaceSyncPayload {
            workspace_id: workspace.id().clone(),
            status: workspace.status(),
            policy: workspace.policy().clone(),
            signing_profile: Some(workspace.signing_profile().clone()),
            api_keys: self.secret_store.list_api_keys(permission.workspace_id())?,
        })
    }
}
