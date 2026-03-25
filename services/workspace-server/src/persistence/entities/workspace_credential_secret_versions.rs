use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "workspace_credential_secret_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub api_key_id: String,
    pub secret_hash: String,
    pub version: i32,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workspace_credentials::Entity",
        from = "Column::ApiKeyId",
        to = "super::workspace_credentials::Column::ApiKeyId",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    WorkspaceCredentials,
}

impl Related<super::workspace_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkspaceCredentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
