use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkspaceMemberships::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkspaceMemberships::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceMemberships::WorkspaceId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceMemberships::UserId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceMemberships::Role)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkspaceMemberships::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-membership-workspace")
                            .from(
                                WorkspaceMemberships::Table,
                                WorkspaceMemberships::WorkspaceId,
                            )
                            .to(Workspaces::Table, Workspaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WorkspaceMemberships::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum WorkspaceMemberships {
    Table,
    Id,
    WorkspaceId,
    UserId,
    Role,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Workspaces {
    Table,
    Id,
}
