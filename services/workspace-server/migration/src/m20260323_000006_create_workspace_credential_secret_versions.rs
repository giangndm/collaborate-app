use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkspaceCredentialSecretVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkspaceCredentialSecretVersions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceCredentialSecretVersions::ApiKeyId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceCredentialSecretVersions::SecretHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceCredentialSecretVersions::Version)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceCredentialSecretVersions::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-secret-version-credential")
                            .from(
                                WorkspaceCredentialSecretVersions::Table,
                                WorkspaceCredentialSecretVersions::ApiKeyId,
                            )
                            .to(WorkspaceCredentials::Table, WorkspaceCredentials::ApiKeyId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(WorkspaceCredentialSecretVersions::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub enum WorkspaceCredentialSecretVersions {
    Table,
    Id,
    ApiKeyId,
    SecretHash,
    Version,
    CreatedAt,
}

#[derive(DeriveIden)]
enum WorkspaceCredentials {
    Table,
    ApiKeyId,
}
