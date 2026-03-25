use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "workspace_credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub api_key_id: String,
    pub workspace_id: String,
    pub label: String,
    pub status: String,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::workspaces::Entity",
        from = "Column::WorkspaceId",
        to = "super::workspaces::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Workspaces,
    #[sea_orm(has_many = "super::workspace_credential_secret_versions::Entity")]
    SecretVersions,
}

impl Related<super::workspaces::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Workspaces.def()
    }
}

impl Related<super::workspace_credential_secret_versions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SecretVersions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
