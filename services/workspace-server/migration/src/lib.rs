pub use sea_orm_migration::prelude::*;

mod m20260323_000001_create_users;
mod m20260323_000002_create_auth_sessions;
mod m20260323_000003_create_workspaces;
mod m20260323_000004_create_workspace_memberships;
mod m20260323_000005_create_workspace_credentials;
mod m20260323_000006_create_workspace_credential_secret_versions;
mod m20260325_000001_add_signing_profile_to_workspaces;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260323_000001_create_users::Migration),
            Box::new(m20260323_000002_create_auth_sessions::Migration),
            Box::new(m20260323_000003_create_workspaces::Migration),
            Box::new(m20260323_000004_create_workspace_memberships::Migration),
            Box::new(m20260323_000005_create_workspace_credentials::Migration),
            Box::new(m20260323_000006_create_workspace_credential_secret_versions::Migration),
            Box::new(m20260325_000001_add_signing_profile_to_workspaces::Migration),
        ]
    }
}
