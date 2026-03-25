use serde::{Deserialize, Serialize};
use crate::auth::AuthenticatedActor;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user: AuthenticatedActor,
}
