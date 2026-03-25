use sea_orm::{Database, DatabaseConnection};
use migration::{Migrator, MigratorTrait};
use anyhow::Result;

pub async fn setup_db(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}
