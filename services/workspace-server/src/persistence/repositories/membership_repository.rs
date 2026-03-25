use async_trait::async_trait;
use core_domain::workspace::{
    MembershipRepository, UserId, WorkspaceError, WorkspaceId, WorkspaceMembership,
    WorkspaceMembershipId, WorkspaceResult, WorkspaceRole,
};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};
use std::str::FromStr;

#[derive(Clone)]
pub struct SqliteMembershipRepository {
    db: DatabaseConnection,
}

impl SqliteMembershipRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MembershipRepository for SqliteMembershipRepository {
    async fn get(
        &self,
        membership_id: &WorkspaceMembershipId,
    ) -> WorkspaceResult<WorkspaceMembership> {
        let model = crate::persistence::entities::workspace_memberships::Entity::find_by_id(
            membership_id.to_string(),
        )
        .one(&self.db)
        .await
        .map_err(|e| WorkspaceError::Internal(e.to_string()))?
        .ok_or_else(|| WorkspaceError::MembershipNotFound {
            membership_id: membership_id.clone(),
        })?;

        map_membership_model_to_domain(model)
    }

    async fn find_for_workspace_user(
        &self,
        workspace_id: &WorkspaceId,
        user_id: &UserId,
    ) -> WorkspaceResult<WorkspaceMembership> {
        let model = crate::persistence::entities::workspace_memberships::Entity::find()
            .filter(
                crate::persistence::entities::workspace_memberships::Column::WorkspaceId
                    .eq(workspace_id.to_string()),
            )
            .filter(
                crate::persistence::entities::workspace_memberships::Column::UserId
                    .eq(user_id.to_string()),
            )
            .one(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?
            .ok_or_else(|| WorkspaceError::MembershipNotFound {
                membership_id: WorkspaceMembershipId::new(format!("{}:{}", workspace_id, user_id)),
            })?;

        map_membership_model_to_domain(model)
    }

    async fn list_for_user(&self, user_id: &UserId) -> WorkspaceResult<Vec<WorkspaceMembership>> {
        let models = crate::persistence::entities::workspace_memberships::Entity::find()
            .filter(
                crate::persistence::entities::workspace_memberships::Column::UserId
                    .eq(user_id.to_string()),
            )
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models
            .into_iter()
            .map(map_membership_model_to_domain)
            .collect()
    }

    async fn list_for_workspace(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<Vec<WorkspaceMembership>> {
        let models = crate::persistence::entities::workspace_memberships::Entity::find()
            .filter(
                crate::persistence::entities::workspace_memberships::Column::WorkspaceId
                    .eq(workspace_id.to_string()),
            )
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models
            .into_iter()
            .map(map_membership_model_to_domain)
            .collect()
    }

    async fn remove(&self, membership_id: &WorkspaceMembershipId) -> WorkspaceResult<()> {
        crate::persistence::entities::workspace_memberships::Entity::delete_by_id(
            membership_id.to_string(),
        )
        .exec(&self.db)
        .await
        .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn save_with_workspace_bump(
        &self,
        membership: &WorkspaceMembership,
    ) -> WorkspaceResult<()> {
        let db = &self.db;
        let txn = db
            .begin()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        self.save_in_txn(membership, &txn).await?;
        self.bump_workspace_in_txn(membership.workspace_id(), &txn)
            .await?;

        txn.commit()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn remove_with_workspace_bump(
        &self,
        workspace_id: &WorkspaceId,
        membership_id: &WorkspaceMembershipId,
    ) -> WorkspaceResult<()> {
        let db = &self.db;
        let txn = db
            .begin()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        crate::persistence::entities::workspace_memberships::Entity::delete_by_id(
            membership_id.to_string(),
        )
        .exec(&txn)
        .await
        .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        self.bump_workspace_in_txn(workspace_id, &txn).await?;

        txn.commit()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn save(&self, membership: &WorkspaceMembership) -> WorkspaceResult<()> {
        self.save_in_txn(membership, &self.db).await
    }
}

impl SqliteMembershipRepository {
    async fn save_in_txn<C>(&self, membership: &WorkspaceMembership, db: &C) -> WorkspaceResult<()>
    where
        C: sea_orm::ConnectionTrait,
    {
        use crate::persistence::entities::workspace_memberships;

        let active_model = workspace_memberships::ActiveModel {
            id: ActiveValue::Set(membership.id().to_string()),
            workspace_id: ActiveValue::Set(membership.workspace_id().to_string()),
            user_id: ActiveValue::Set(membership.user_id().to_string()),
            role: ActiveValue::Set(membership.role().to_string()),
            ..Default::default()
        };

        workspace_memberships::Entity::insert(active_model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(workspace_memberships::Column::Id)
                    .update_columns([workspace_memberships::Column::Role])
                    .to_owned(),
            )
            .exec(db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn bump_workspace_in_txn<C>(
        &self,
        workspace_id: &WorkspaceId,
        db: &C,
    ) -> WorkspaceResult<()>
    where
        C: sea_orm::ConnectionTrait,
    {
        use crate::persistence::entities::workspaces;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

        workspaces::Entity::update_many()
            .col_expr(
                workspaces::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(chrono::Utc::now()),
            )
            .filter(workspaces::Column::Id.eq(workspace_id.to_string()))
            .exec(db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(())
    }
}

fn map_membership_model_to_domain(
    model: crate::persistence::entities::workspace_memberships::Model,
) -> WorkspaceResult<WorkspaceMembership> {
    let role = WorkspaceRole::from_str(&model.role)
        .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

    Ok(WorkspaceMembership::new(
        WorkspaceMembershipId::new(model.id),
        WorkspaceId::new(model.workspace_id),
        UserId::new(model.user_id),
        role,
    ))
}
