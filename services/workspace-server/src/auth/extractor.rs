use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use axum_extra::extract::CookieJar;
use crate::auth::AuthenticatedActor;
use crate::http::error::HttpError;
use crate::app::state::AppState;
use core_domain::workspace::{UserId, UserRepository};

impl FromRequestParts<AppState> for AuthenticatedActor
{
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let cookies = CookieJar::from_headers(&parts.headers);
        let session_id = cookies
            .get("workspace_console_session")
            .map(|c| c.value().to_string());

        if let Some(sid) = session_id {
            if let Ok(Some(session)) = state.auth_session_repo.find_session(&sid).await {
                if !session.is_expired() {
                    let user_id = UserId::from(session.user_id);
                    if let Ok(user) = state.user_repo.get(&user_id).await {
                        return Ok(AuthenticatedActor {
                            user_id: user.id().clone(),
                            global_role: user.role().clone(),
                            display_name: user.profile().display_name().to_string(),
                            email: user.profile().email().to_string(),
                        });
                    }
                }
            }
        }

        Err(HttpError::Unauthorized)
    }
}
