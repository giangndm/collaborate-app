use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkspaceCredentials::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkspaceCredentials::ApiKeyId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(WorkspaceCredentials::WorkspaceId).string().not_null())
                    .col(ColumnDef::new(WorkspaceCredentials::Label).string().not_null())
                    .col(ColumnDef::new(WorkspaceCredentials::Status).string().not_null())
                    .col(
                        ColumnDef::new(WorkspaceCredentials::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-credential-workspace")
                            .from(WorkspaceCredentials::Table, WorkspaceCredentials::WorkspaceId)
                            .to(Workspaces::Table, Workspaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkspaceCredentials::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum WorkspaceCredentials {
    Table,
    ApiKeyId,
    WorkspaceId,
    Label,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
}
