use crate::app::state::AppState;
use crate::auth::AuthenticatedActor;
use crate::http::dto::credentials::{
    CreateCredentialRequest, CredentialMetadataDto, CredentialSecretDto,
};
use crate::http::error::HttpError;
use crate::persistence::repositories::{
    SqliteMembershipRepository, SqliteSecretStore, SqliteUserRepository, SqliteWorkspaceRepository,
};
use axum::{
    Json, Router,
    extract::{Path, State},
};
use core_domain::workspace::{
    User, WorkspaceApiKeyId, WorkspaceId, WorkspaceReadPermission, WorkspaceService,
    WorkspaceWritePermission,
};

pub async fn list_credentials(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<CredentialMetadataDto>>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let permission = WorkspaceReadPermission::new(ws_id);

    let credentials = service.list_credentials(&permission).await.map_err(|e| {
        eprintln!("List credentials error: {:?}", e);
        HttpError::InternalServerError
    })?;

    Ok(Json(credentials.into_iter().map(Into::into).collect()))
}

pub async fn create_credential(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path(workspace_id): Path<String>,
    Json(payload): Json<CreateCredentialRequest>,
) -> Result<Json<CredentialSecretDto>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let user = actor_to_user(&state, &actor).await?;
    let permission = WorkspaceWritePermission::new(ws_id);

    let secret = service
        .create_credential(&permission, &user, &payload.label)
        .await
        .map_err(|e| {
            eprintln!("Create credential error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(secret.into()))
}

pub async fn rotate_credential(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path((workspace_id, api_key_id)): Path<(String, String)>,
) -> Result<Json<CredentialSecretDto>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let key_id = WorkspaceApiKeyId(api_key_id);
    let user = actor_to_user(&state, &actor).await?;
    let permission = WorkspaceWritePermission::new(ws_id);

    let secret = service
        .rotate_secret(&permission, &user, &key_id)
        .await
        .map_err(|e| {
            eprintln!("Rotate credential error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(secret.into()))
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

async fn actor_to_user(state: &AppState, actor: &AuthenticatedActor) -> Result<User, HttpError> {
    use core_domain::workspace::UserRepository;
    state
        .user_repo
        .get(&actor.user_id)
        .await
        .map_err(|_| HttpError::Unauthorized)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/workspaces/{id}/credentials",
            axum::routing::get(list_credentials).post(create_credential),
        )
        .route(
            "/workspaces/{id}/credentials/{key_id}/rotate",
            axum::routing::post(rotate_credential),
        )
}
