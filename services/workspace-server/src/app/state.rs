use crate::config::AppConfig;
use crate::persistence::repositories::{
    SqliteUserRepository, SqliteWorkspaceRepository, SqliteMembershipRepository, SqliteAuthSessionRepository,
    SqliteSecretStore
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub db: DatabaseConnection,
    pub user_repo: Arc<SqliteUserRepository>,
    pub workspace_repo: Arc<SqliteWorkspaceRepository>,
    pub membership_repo: Arc<SqliteMembershipRepository>,
    pub auth_session_repo: Arc<SqliteAuthSessionRepository>,
    pub secret_store: Arc<SqliteSecretStore>,
}

impl AppState {
    pub fn new(config: AppConfig, db: DatabaseConnection) -> Self {
        Self {
            config: Arc::new(config),
            db: db.clone(),
            user_repo: Arc::new(SqliteUserRepository::new(db.clone())),
            workspace_repo: Arc::new(SqliteWorkspaceRepository::new(db.clone())),
            membership_repo: Arc::new(SqliteMembershipRepository::new(db.clone())),
            auth_session_repo: Arc::new(SqliteAuthSessionRepository::new(db.clone())),
            secret_store: Arc::new(SqliteSecretStore::new(db.clone())),
        }
    }
}
