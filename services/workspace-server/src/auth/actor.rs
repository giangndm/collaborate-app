use core_domain::workspace::{GlobalUserRole, UserId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedActor {
    pub user_id: UserId,
    pub global_role: GlobalUserRole,
    pub display_name: String,
    pub email: String,
}
