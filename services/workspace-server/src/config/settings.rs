use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_assets_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub mock_accounts: Vec<MockAccount>,
    pub session_ttl_days: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MockAccount {
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub global_role: String,
}

impl AppConfig {
    pub fn build(cli: super::Cli) -> anyhow::Result<Self> {
        let mut builder = config::Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 3000)?
            .set_default("database.url", "sqlite:console.db")?
            .set_default("auth.session_ttl_days", 7)?;

        if let Some(config_path) = cli.config {
            builder = builder.add_source(config::File::from(config_path));
        }

        builder = builder.add_source(config::Environment::with_prefix("CONSOLE").separator("__"));

        if let Some(db_url) = cli.database_url {
            builder = builder.set_override("database.url", db_url)?;
        }

        if let Some(port) = cli.port {
            builder = builder.set_override("server.port", port)?;
        }

        let config = builder.build()?;
        Ok(config.try_deserialize()?)
    }
}
