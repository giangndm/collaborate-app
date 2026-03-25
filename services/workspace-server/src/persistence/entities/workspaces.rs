use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "workspaces")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
    pub guest_join_enabled: bool,
    pub token_ttl_seconds: i32,
    pub active_signing_secret_id: Option<String>,
    pub active_signing_secret_version: Option<i64>,
    pub guest_access: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::workspace_memberships::Entity")]
    WorkspaceMemberships,
    #[sea_orm(has_many = "super::workspace_credentials::Entity")]
    WorkspaceCredentials,
}

impl Related<super::workspace_memberships::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkspaceMemberships.def()
    }
}

impl Related<super::workspace_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkspaceCredentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
