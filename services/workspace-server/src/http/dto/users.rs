use core_domain::workspace::{GlobalUserRole, User, UserStatus};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserDto {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub global_role: String,
    pub status: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id().to_string(),
            email: user.profile().email().to_string(),
            display_name: user.profile().display_name().to_string(),
            global_role: user.role().to_string(),
            status: user.status().to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub global_role: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub global_role: Option<String>,
    pub status: Option<String>,
}
