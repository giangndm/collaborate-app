use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub global_role: String,
    pub status: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::workspace_memberships::Entity")]
    WorkspaceMemberships,
}

impl Related<super::workspace_memberships::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkspaceMemberships.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
