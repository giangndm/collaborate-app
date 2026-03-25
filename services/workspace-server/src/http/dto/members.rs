use serde::{Deserialize, Serialize};
use core_domain::workspace::WorkspaceMemberView;

#[derive(Serialize)]
pub struct WorkspaceMemberDto {
    pub membership_id: String,
    pub user_id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
}

impl From<WorkspaceMemberView> for WorkspaceMemberDto {
    fn from(view: WorkspaceMemberView) -> Self {
        Self {
            membership_id: view.membership_id.to_string(),
            user_id: view.user_id.to_string(),
            display_name: view.display_name.to_string(),
            email: view.email.to_string(),
            role: view.workspace_role.to_string(),
        }
    }
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: String,
}

#[derive(Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}
