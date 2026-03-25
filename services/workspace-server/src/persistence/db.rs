use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};

pub async fn setup_db(db_url: &str) -> Result<DatabaseConnection> {
    let db = Database::connect(db_url).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}
