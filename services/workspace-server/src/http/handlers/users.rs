use crate::app::state::AppState;
use crate::auth::AuthenticatedActor;
use crate::http::dto::users::{CreateUserRequest, UpdateUserRequest, UserDto};
use crate::http::error::HttpError;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use core_domain::workspace::{
    DisplayName, GlobalUserRole, User, UserEmail, UserId, UserProfile, UserStatus,
};
use std::str::FromStr;

pub async fn list_users(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
) -> Result<Json<Vec<UserDto>>, HttpError> {
    let service = create_service(&state);
    let actor_user = actor_to_user(&state, &actor).await?;

    let users = service.list_all_users(&actor_user).await.map_err(|e| {
        eprintln!("List users error: {:?}", e);
        HttpError::InternalServerError
    })?;

    Ok(Json(users.into_iter().map(Into::into).collect()))
}

pub async fn get_user(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path(user_id): Path<String>,
) -> Result<Json<UserDto>, HttpError> {
    let service = create_service(&state);
    let actor_user = actor_to_user(&state, &actor).await?;
    let target_id = UserId(user_id);

    let user = service
        .get_user(&actor_user, &target_id)
        .await
        .map_err(|e| {
            eprintln!("Get user error: {:?}", e);
            HttpError::InternalServerError
        })?;

    Ok(Json(user.into()))
}

pub async fn create_user(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserDto>, HttpError> {
    let service = create_service(&state);
    let actor_user = actor_to_user(&state, &actor).await?;

    let role = GlobalUserRole::from_str(&payload.global_role)
        .map_err(|_| HttpError::BadRequest("Invalid global role".to_string()))?;

    let status = UserStatus::from_str(&payload.status)
        .map_err(|_| HttpError::BadRequest("Invalid status".to_string()))?;

    let user = User::from_persistence(
        UserId(payload.id),
        role,
        status,
        UserProfile::new(
            UserEmail::new(payload.email),
            DisplayName::new(payload.display_name),
        ),
    );

    service.save_user(&actor_user, &user).await.map_err(|e| {
        eprintln!("Create user error: {:?}", e);
        HttpError::InternalServerError
    })?;

    Ok(Json(user.into()))
}

pub async fn update_user(
    State(state): State<AppState>,
    actor: AuthenticatedActor,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserDto>, HttpError> {
    let service = create_service(&state);
    let actor_user = actor_to_user(&state, &actor).await?;
    let target_id = UserId(user_id);

    let mut user = service
        .get_user(&actor_user, &target_id)
        .await
        .map_err(|_| HttpError::NotFound("User not found".to_string()))?;

    // This is a bit simplified, ideally we have more granular update methods
    // But for now we rebuild the User object if we were to support full updates
    // Actually, User entity doesn't have public setters for role/profile yet
    // Let's just implement what we can or skip for now if too complex.
    // For this task, we mainly need List and Create to support member addition.

    // If we wanted to update status:
    if let Some(status_str) = payload.status {
        let status = UserStatus::from_str(&status_str)
            .map_err(|_| HttpError::BadRequest("Invalid status".to_string()))?;
        match status {
            UserStatus::Active => user.activate(),
            UserStatus::Suspended => user.suspend(),
            UserStatus::Disabled => user.disable(),
        }
    }

    service.save_user(&actor_user, &user).await.map_err(|e| {
        eprintln!("Update user error: {:?}", e);
        HttpError::InternalServerError
    })?;

    Ok(Json(user.into()))
}

fn create_service(
    state: &AppState,
) -> core_domain::workspace::WorkspaceService<
    crate::persistence::repositories::SqliteWorkspaceRepository,
    crate::persistence::repositories::SqliteUserRepository,
    crate::persistence::repositories::SqliteMembershipRepository,
    crate::persistence::repositories::SqliteSecretStore,
> {
    core_domain::workspace::WorkspaceService::new(
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
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/{id}",
            get(get_user).patch(update_user).put(update_user),
        )
}
