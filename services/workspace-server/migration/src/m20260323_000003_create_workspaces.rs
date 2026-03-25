use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Workspaces::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Workspaces::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Workspaces::Name).string().not_null())
                    .col(
                        ColumnDef::new(Workspaces::Slug)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Workspaces::Status).string().not_null())
                    .col(
                        ColumnDef::new(Workspaces::GuestJoinEnabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Workspaces::TokenTtlSeconds)
                            .integer()
                            .not_null()
                            .default(3600),
                    )
                    .col(
                        ColumnDef::new(Workspaces::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Workspaces::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Workspaces::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Workspaces {
    Table,
    Id,
    Name,
    Slug,
    Status,
    GuestJoinEnabled,
    TokenTtlSeconds,
    CreatedAt,
    UpdatedAt,
}
