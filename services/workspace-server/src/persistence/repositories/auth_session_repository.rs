use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};
use crate::persistence::entities::auth_sessions;
use crate::auth::Session;
use anyhow::Result;
use chrono::Utc;

pub struct SqliteAuthSessionRepository {
    db: DatabaseConnection,
}

impl SqliteAuthSessionRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn create_session(&self, user_id: &str, ttl_days: u32) -> Result<Session> {
        let session = Session::new(user_id.to_string(), ttl_days);
        
        let active_model = auth_sessions::ActiveModel {
            id: ActiveValue::Set(session.id.clone()),
            user_id: ActiveValue::Set(session.user_id.clone()),
            expires_at: ActiveValue::Set(session.expires_at),
            created_at: ActiveValue::Set(Utc::now()),
        };

        active_model.insert(&self.db).await?;
        
        Ok(session)
    }

    pub async fn find_session(&self, session_id: &str) -> Result<Option<Session>> {
        let model = auth_sessions::Entity::find_by_id(session_id.to_string())
            .one(&self.db)
            .await?;
            
        Ok(model.map(|m| Session {
            id: m.id,
            user_id: m.user_id,
            expires_at: m.expires_at,
        }))
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        auth_sessions::Entity::delete_by_id(session_id.to_string())
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
