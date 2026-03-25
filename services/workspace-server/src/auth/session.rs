use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    pub fn new(user_id: String, ttl_days: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            expires_at: Utc::now() + chrono::Duration::days(ttl_days as i64),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}
