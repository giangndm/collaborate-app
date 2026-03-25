use axum::{
    extract::{Path, State},
    routing::{get, post, delete, patch},
    Json, Router,
};
use core_domain::workspace::{
    User, UserId, WorkspaceId, WorkspaceMemberView, WorkspaceMembership, WorkspaceMembershipId,
    WorkspaceReadPermission, WorkspaceRole, WorkspaceService, WorkspaceWritePermission,
    UserRepository
};
use crate::app::state::AppState;
use crate::auth::AuthenticatedActor;
use crate::http::dto::members::{AddMemberRequest, UpdateMemberRoleRequest, WorkspaceMemberDto};
use crate::http::error::HttpError;
use std::str::FromStr;

pub async fn list_members(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<WorkspaceMemberDto>>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let permission = WorkspaceReadPermission::new(ws_id);
    
    let members = service
        .list_members(&permission)
        .await
        .map_err(|e| {
            eprintln!("List members error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(members.into_iter().map(Into::into).collect()))
}

pub async fn add_member(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path(workspace_id): Path<String>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<WorkspaceMemberDto>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let user_id = UserId(payload.user_id);
    let role = WorkspaceRole::from_str(&payload.role)
        .map_err(|_| HttpError::BadRequest("Invalid role".to_string()))?;
    
    let actor_user = actor_to_user(&state, &actor).await?;
    let permission = WorkspaceWritePermission::new(ws_id.clone());

    let membership = WorkspaceMembership::new(
        WorkspaceMembershipId(format!("{}:{}", ws_id, user_id)),
        ws_id.clone(),
        user_id,
        role,
    );

    let view = service
        .add_member(&permission, &actor_user, &membership)
        .await
        .map_err(|e| {
            eprintln!("Add member error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(view.into()))
}

pub async fn remove_member(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path((workspace_id, target_user_id)): Path<(String, String)>,
) -> Result<Json<()>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let t_user_id = UserId(target_user_id);
    
    let actor_user = actor_to_user(&state, &actor).await?;
    let permission = WorkspaceWritePermission::new(ws_id);

    service
        .remove_member(&permission, &actor_user, &t_user_id)
        .await
        .map_err(|e| {
            eprintln!("Remove member error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(()))
}

pub async fn update_member_role(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path((workspace_id, target_user_id)): Path<(String, String)>,
    Json(payload): Json<UpdateMemberRoleRequest>,
) -> Result<Json<WorkspaceMemberDto>, HttpError> {
    let service = create_service(&state);
    let ws_id = WorkspaceId(workspace_id);
    let t_user_id = UserId(target_user_id);
    let role = WorkspaceRole::from_str(&payload.role)
        .map_err(|_| HttpError::BadRequest("Invalid role".to_string()))?;
    
    let actor_user = actor_to_user(&state, &actor).await?;
    let permission = WorkspaceWritePermission::new(ws_id);

    let view = service
        .change_member_role(&permission, &actor_user, &t_user_id, role)
        .await
        .map_err(|e| {
            eprintln!("Change member role error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(view.into()))
}

fn create_service(state: &AppState) -> WorkspaceService<
    crate::persistence::repositories::SqliteWorkspaceRepository,
    crate::persistence::repositories::SqliteUserRepository,
    crate::persistence::repositories::SqliteMembershipRepository,
    crate::persistence::repositories::SqliteSecretStore
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
    state.user_repo.get(&actor.user_id).await
        .map_err(|_| HttpError::Unauthorized)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/workspaces/{id}/members", get(list_members).post(add_member))
        .route("/workspaces/{id}/members/{user_id}", delete(remove_member).patch(update_member_role))
}
