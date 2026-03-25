use async_trait::async_trait;
use core_domain::workspace::{
    DefaultRoomPolicy, Workspace, WorkspaceError, WorkspaceId, WorkspaceLastUpdated, WorkspaceName,
    WorkspacePolicy, WorkspaceRepository, WorkspaceResult, WorkspaceSigningProfile, WorkspaceSlug,
    WorkspaceStatus, WorkspaceSummary, WorkspaceSecretRef, WorkspaceSecretRefId, WorkspaceSecretVersion
};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, ActiveModelTrait};
use std::str::FromStr;

#[derive(Clone)]
pub struct SqliteWorkspaceRepository {
    db: DatabaseConnection,
}

impl SqliteWorkspaceRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl WorkspaceRepository for SqliteWorkspaceRepository {
    async fn get(&self, id: &WorkspaceId) -> WorkspaceResult<Workspace> {
        let model = crate::persistence::entities::workspaces::Entity::find_by_id(id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound {
                workspace_id: id.clone(),
            })?;

        map_workspace_model_to_domain(model)
    }

    async fn list_all(&self) -> WorkspaceResult<Vec<Workspace>> {
        let models = crate::persistence::entities::workspaces::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models
            .into_iter()
            .map(map_workspace_model_to_domain)
            .collect()
    }

    async fn list_for_ids(&self, ids: &[WorkspaceId]) -> WorkspaceResult<Vec<Workspace>> {
        let id_strs: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let models = crate::persistence::entities::workspaces::Entity::find()
            .filter(crate::persistence::entities::workspaces::Column::Id.is_in(id_strs))
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models
            .into_iter()
            .map(map_workspace_model_to_domain)
            .collect()
    }

    async fn create_with_owner(
        &self,
        workspace: &Workspace,
        owner_membership: &core_domain::workspace::WorkspaceMembership,
    ) -> WorkspaceResult<()> {
        let _ = owner_membership; // Membership creation should be handled in a transaction ideally
        self.save(workspace).await
    }

    async fn save(&self, workspace: &Workspace) -> WorkspaceResult<()> {
        use crate::persistence::entities::workspaces;
        
        let signing_profile = workspace.signing_profile();
        
        let active_model = workspaces::ActiveModel {
            id: ActiveValue::Set(workspace.id().to_string()),
            name: ActiveValue::Set(workspace.name().to_string()),
            slug: ActiveValue::Set(workspace.slug().to_string()),
            status: ActiveValue::Set(workspace.status().to_string()),
            guest_join_enabled: ActiveValue::Set(workspace.default_room_policy().guest_join_enabled),
            token_ttl_seconds: ActiveValue::Set(workspace.default_room_policy().token_ttl_seconds as i32),
            active_signing_secret_id: ActiveValue::Set(Some(signing_profile.active_secret_ref.secret_ref_id.to_string())),
            active_signing_secret_version: ActiveValue::Set(Some(signing_profile.active_secret_ref.version.get() as i64)),
            guest_access: ActiveValue::Set(workspace.policy().guest_access.to_string()),
            updated_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        workspaces::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(workspaces::Column::Id)
                    .update_columns([
                        workspaces::Column::Name,
                        workspaces::Column::Slug,
                        workspaces::Column::Status,
                        workspaces::Column::GuestJoinEnabled,
                        workspaces::Column::TokenTtlSeconds,
                        workspaces::Column::ActiveSigningSecretId,
                        workspaces::Column::ActiveSigningSecretVersion,
                        workspaces::Column::GuestAccess,
                        workspaces::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn map_workspace_model_to_domain(
    model: crate::persistence::entities::workspaces::Model,
) -> WorkspaceResult<Workspace> {
    let status = WorkspaceStatus::from_str(&model.status)
        .map_err(|e| WorkspaceError::Internal(e))?;

    let policy = WorkspacePolicy {
        guest_access: GuestAccessPolicy::from_str(&model.guest_access)
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?,
    };

    let default_room_policy = DefaultRoomPolicy {
        guest_join_enabled: model.guest_join_enabled,
        token_ttl_seconds: model.token_ttl_seconds as u32,
    };

    let signing_profile = WorkspaceSigningProfile {
        active_secret_ref: WorkspaceSecretRef {
            secret_ref_id: WorkspaceSecretRefId::new(
                model
                    .active_signing_secret_id
                    .unwrap_or_else(|| "default_secret".to_string()),
            ),
            version: WorkspaceSecretVersion::new(
                model.active_signing_secret_version.unwrap_or(1) as u64
            ),
        },
    };

    Ok(Workspace::rehydrate(
        WorkspaceId::new(model.id),
        WorkspaceName::new(model.name),
        WorkspaceSlug::new(model.slug),
        status,
        policy,
        default_room_policy,
        WorkspaceLastUpdated::from_rfc3339(&model.updated_at.to_rfc3339())
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?,
        signing_profile,
    ))
}

use core_domain::workspace::GuestAccessPolicy;
