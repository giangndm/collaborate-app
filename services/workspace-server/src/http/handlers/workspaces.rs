use crate::app::state::AppState;
use crate::auth::AuthenticatedActor;
use crate::http::dto::workspaces::{
    CreateWorkspaceRequest, UpdateWorkspaceRequest, WorkspaceDetailDto, WorkspaceSummaryDto,
};
use crate::http::error::HttpError;
use crate::persistence::repositories::{
    SqliteMembershipRepository, SqliteSecretStore, SqliteUserRepository, SqliteWorkspaceRepository,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use core_domain::workspace::{
    DefaultRoomPolicy, User, Workspace, WorkspaceCreatorGuard, WorkspaceId, WorkspaceName,
    WorkspacePolicy, WorkspaceReadPermission, WorkspaceSecretRef, WorkspaceSecretRefId,
    WorkspaceSecretVersion, WorkspaceService, WorkspaceSigningProfile, WorkspaceSlug,
    WorkspaceStatus, WorkspaceUpdate, WorkspaceWritePermission,
};
use std::str::FromStr;

pub async fn list_workspaces(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
) -> Result<Json<Vec<WorkspaceSummaryDto>>, HttpError> {
    let service = create_service(&state);
    let user = actor_to_user(&state, &actor).await?;

    let summaries = service
        .list_workspaces_visible_to_actor(&user)
        .await
        .map_err(|e| {
            eprintln!("List workspaces error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(summaries.into_iter().map(Into::into).collect()))
}

pub async fn create_workspace(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<Json<WorkspaceDetailDto>, HttpError> {
    let service = create_service(&state);
    let user = actor_to_user(&state, &actor).await?;

    let guard = WorkspaceCreatorGuard::try_from_actor(&user).ok_or(HttpError::Forbidden)?;

    let workspace = Workspace::new(
        WorkspaceId(payload.id),
        WorkspaceName::new(payload.name),
        WorkspaceSlug::new(payload.slug),
        WorkspaceStatus::Active,
        WorkspacePolicy::default(),
        DefaultRoomPolicy::new(true, 3600),
        WorkspaceSigningProfile {
            active_secret_ref: WorkspaceSecretRef {
                secret_ref_id: WorkspaceSecretRefId("initial_secret".to_string()),
                version: WorkspaceSecretVersion(1),
            },
        },
    );

    let _membership = service
        .create_workspace(&guard, &workspace)
        .await
        .map_err(|e| {
            eprintln!("Create workspace error: {:?}", e);
            HttpError::InternalServerError
        })?;

    let detail = service
        .get_workspace_detail(&WorkspaceReadPermission::new(workspace.id().clone()))
        .await
        .map_err(|_| HttpError::InternalServerError)?;

    Ok(Json(detail.into()))
}

pub async fn get_workspace(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(id): Path<String>,
) -> Result<Json<WorkspaceDetailDto>, HttpError> {
    let service = create_service(&state);
    let workspace_id = WorkspaceId(id);
    let permission = WorkspaceReadPermission::new(workspace_id);

    let detail = service
        .get_workspace_detail(&permission)
        .await
        .map_err(|_| HttpError::NotFound("Workspace not found".to_string()))?;

    Ok(Json(detail.into()))
}

pub async fn update_workspace(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWorkspaceRequest>,
) -> Result<Json<WorkspaceDetailDto>, HttpError> {
    let service = create_service(&state);
    let user = actor_to_user(&state, &actor).await?;
    let workspace_id = WorkspaceId(id);
    let permission = WorkspaceWritePermission::new(workspace_id);

    let status = WorkspaceStatus::from_str(&payload.status)
        .map_err(|_| HttpError::BadRequest("Invalid status".to_string()))?;

    let update = WorkspaceUpdate::new(
        WorkspaceName::new(payload.name),
        status,
        DefaultRoomPolicy::new(payload.guest_join_enabled, payload.token_ttl_seconds),
    );

    let detail = service
        .update_workspace(&permission, &user, update)
        .await
        .map_err(|e| {
            eprintln!("Update workspace error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(detail.into()))
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
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route("/workspaces/{id}", get(get_workspace).put(update_workspace))
}
