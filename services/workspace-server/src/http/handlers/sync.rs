use axum::{
    extract::{Path, State},
    Json, Router,
};
use core_domain::workspace::{
    WorkspaceId, WorkspaceReadPermission, WorkspaceService,
};
use crate::app::state::AppState;
use crate::auth::AuthenticatedActor;
use crate::http::dto::sync::WorkspaceSyncDto;
use crate::http::error::HttpError;
use crate::persistence::repositories::{
    SqliteMembershipRepository, SqliteSecretStore, SqliteUserRepository, SqliteWorkspaceRepository,
};

pub async fn get_sync_payload(
    State(state): State<AppState>,
    _actor: AuthenticatedActor, // Permission check is handled by the guard logic
    Path(workspace_id): Path<String>,
) -> Result<Json<WorkspaceSyncDto>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let permission = WorkspaceReadPermission::new(ws_id);

    let workspace = service.read_workspace(&permission).await
        .map_err(|_| HttpError::NotFound)?;
    
    let credentials = service.list_credentials(&permission).await
        .map_err(|_| HttpError::InternalServerError)?;

    let payload = core_domain::workspace::WorkspaceSyncPayload {
        workspace_id: workspace.id().clone(),
        status: workspace.status(),
        last_updated: workspace.last_updated(),
        policy: workspace.policy().clone(),
        default_room_policy: workspace.default_room_policy().clone(),
        credential_verifiers: credentials
            .iter()
            .map(core_domain::workspace::WorkspaceCredentialVerifier::from_metadata)
            .collect(),
    };

    Ok(Json(payload.into()))
}

fn create_service(
    state: &AppState,
) -> WorkspaceService<
    SqliteWorkspaceRepository,
    SqliteUserRepository,
    SqliteMembershipRepository,
    SqliteSecretStore,
> {
    WorkspaceService::new(
        (*state.workspace_repo).clone(),
        (*state.user_repo).clone(),
        (*state.membership_repo).clone(),
        (*state.secret_store).clone(),
    )
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces/{id}/sync", axum::routing::get(get_sync_payload))
}
