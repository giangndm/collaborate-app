use serde::{Deserialize, Serialize};
use core_domain::workspace::{WorkspaceDetail, WorkspaceSummary};

#[derive(Serialize)]
pub struct WorkspaceSummaryDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
}

impl From<WorkspaceSummary> for WorkspaceSummaryDto {
    fn from(summary: WorkspaceSummary) -> Self {
        Self {
            id: summary.workspace_id.to_string(),
            name: summary.name.to_string(),
            slug: summary.slug.to_string(),
            status: summary.status.to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct WorkspaceDetailDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub guest_join_enabled: bool,
    pub token_ttl_seconds: u32,
}

impl From<WorkspaceDetail> for WorkspaceDetailDto {
    fn from(detail: WorkspaceDetail) -> Self {
        Self {
            id: detail.workspace_id.to_string(),
            name: detail.name.to_string(),
            slug: detail.slug.to_string(),
            status: detail.status.to_string(),
            guest_join_enabled: detail.default_room_policy.guest_join_enabled,
            token_ttl_seconds: detail.default_room_policy.token_ttl_seconds,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: String,
    pub status: String,
    pub guest_join_enabled: bool,
    pub token_ttl_seconds: u32,
}
