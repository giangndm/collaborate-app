use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde::{Deserialize, Serialize};
use crate::auth::AuthenticatedActor;
use crate::app::state::AppState;
use crate::http::error::HttpError;
use crate::auth::password::verify_password;

use crate::http::dto::auth::{LoginRequest, LoginResponse};

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), HttpError> {
    let result = state.user_repo.find_by_email(&payload.email).await
        .map_err(|_| HttpError::InternalServerError)?;
        
    let (user, password_hash) = match result {
        Some((u, h)) => (u, h),
        None => return Err(HttpError::Unauthorized),
    };

    if !verify_password(&payload.password, &password_hash).map_err(|_| HttpError::Unauthorized)? {
        return Err(HttpError::Unauthorized);
    }

    let session = state.auth_session_repo.create_session(&user.id().to_string(), state.config.auth.session_ttl_days).await
        .map_err(|_| HttpError::InternalServerError)?;

    let cookie = Cookie::build(("workspace_console_session", session.id.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .permanent()
        .build();

    let jar = jar.add(cookie);
    
    let actor = AuthenticatedActor {
        user_id: user.id().clone(),
        global_role: user.role().clone(),
        display_name: user.profile().display_name().to_string(),
        email: user.profile().email().to_string(),
    };

    Ok((jar, Json(LoginResponse { user: actor })))
}

pub async fn logout(
    jar: CookieJar,
) -> (CookieJar, Json<String>) {
    let jar = jar.remove(Cookie::from("workspace_console_session"));
    (jar, Json("Logged out".to_string()))
}

pub async fn me(
    actor: AuthenticatedActor,
) -> Json<AuthenticatedActor> {
    Json(actor)
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}
