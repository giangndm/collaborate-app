use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AuthSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuthSessions::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthSessions::UserId).string().not_null())
                    .col(
                        ColumnDef::new(AuthSessions::ExpiresAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AuthSessions::CreatedAt)
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
            .drop_table(Table::drop().table(AuthSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum AuthSessions {
    Table,
    Id,
    UserId,
    ExpiresAt,
    CreatedAt,
}
