use async_trait::async_trait;
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, QuerySelect, Condition, QueryOrder};
use sea_orm::sea_query::{Expr, Query};
use std::str::FromStr;
use core_domain::workspace::{
    UserId, WorkspaceId, User, UserProfile, UserEmail, DisplayName, 
    UserRepository, WorkspaceResult, WorkspaceError, GlobalUserRole, UserStatus
};

#[derive(Clone)]
pub struct SqliteUserRepository {
    db: DatabaseConnection,
}

impl SqliteUserRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn find_by_email(&self, email: &str) -> WorkspaceResult<Option<(User, String)>> {
        let model = crate::persistence::entities::users::Entity::find()
            .filter(crate::persistence::entities::users::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        match model {
            Some(m) => {
                let password_hash = m.password_hash.clone();
                let user = map_user_model_to_domain(m)?;
                Ok(Some((user, password_hash)))
            }
            None => Ok(None),
        }
    }
}

// TODO: Implement actual mapping and SeaORM calls
// This is a skeleton to satisfy the port requirements
#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn get(&self, user_id: &UserId) -> WorkspaceResult<User> {
        let model = crate::persistence::entities::users::Entity::find_by_id(user_id.to_string())
            .one(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?
            .ok_or_else(|| WorkspaceError::UserNotFound { user_id: user_id.clone() })?;

        map_user_model_to_domain(model)
    }

    async fn list_by_ids(&self, user_ids: &[UserId]) -> WorkspaceResult<Vec<User>> {
        let ids: Vec<String> = user_ids.iter().map(|id| id.to_string()).collect();
        
        let models = crate::persistence::entities::users::Entity::find()
            .filter(crate::persistence::entities::users::Column::Id.is_in(ids))
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models.into_iter().map(map_user_model_to_domain).collect()
    }

    async fn find_candidates(
        &self,
        workspace_id: &WorkspaceId,
        query: &str,
        limit: usize,
    ) -> WorkspaceResult<Vec<User>> {
        use crate::persistence::entities::{users, workspace_memberships};
        
        // Find users who are NOT already in the workspace
        let members_subquery = Query::select()
            .column(workspace_memberships::Column::UserId)
            .from(workspace_memberships::Entity)
            .and_where(Expr::col(workspace_memberships::Column::WorkspaceId).eq(workspace_id.to_string()))
            .to_owned();

        let mut find = users::Entity::find()
            .filter(Expr::col(users::Column::Id).not_in_subquery(members_subquery));

        if !query.is_empty() {
            let q = format!("%{}%", query);
            find = find.filter(
                Condition::any()
                    .add(users::Column::DisplayName.like(&q))
                    .add(users::Column::Email.like(&q))
            );
        }

        let models: Vec<users::Model> = find
            .limit(limit as u64)
            .order_by_asc(users::Column::DisplayName)
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        models.into_iter().map(map_user_model_to_domain).collect()
    }
}

fn map_user_model_to_domain(model: crate::persistence::entities::users::Model) -> WorkspaceResult<User> {
    let role = GlobalUserRole::from_str(&model.global_role)
        .map_err(|e| WorkspaceError::Internal(e))?;
    
    let status = UserStatus::from_str(&model.status)
        .map_err(|e| WorkspaceError::Internal(e))?;

    let profile = UserProfile::new(
        UserEmail::new(model.email),
        DisplayName::new(model.display_name),
    );

    Ok(User::from_persistence(
        UserId::from(model.id),
        role,
        status,
        profile,
    ))
}
