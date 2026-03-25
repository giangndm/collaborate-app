use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};
use anyhow::Result;
use crate::persistence::entities::{users, workspaces, workspace_memberships};
use crate::auth::password::hash_password;
use core_domain::workspace::{GlobalUserRole, WorkspaceStatus, GuestAccessPolicy, WorkspaceRole, UserStatus};
use chrono::Utc;

pub async fn seed_dev_data(db: &DatabaseConnection) -> Result<()> {
    // 1. Create Super Admin
    let admin_id = "user_admin".to_string();
    let admin_exists = users::Entity::find_by_id(admin_id.clone()).one(db).await?.is_some();
    if !admin_exists {
        let admin = users::ActiveModel {
            id: ActiveValue::Set(admin_id.clone()),
            email: ActiveValue::Set("admin@example.com".to_string()),
            password_hash: ActiveValue::Set(hash_password("password")?),
            display_name: ActiveValue::Set("Super Admin".to_string()),
            global_role: ActiveValue::Set(GlobalUserRole::SuperAdmin.to_string()),
            status: ActiveValue::Set(UserStatus::Active.to_string()),
            created_at: ActiveValue::Set(Utc::now()),
            updated_at: ActiveValue::Set(Utc::now()),
        };
        users::Entity::insert(admin).exec(db).await?;
    }

    // 2. Create Regular Member
    let member_id = "user_member".to_string();
    let member_exists = users::Entity::find_by_id(member_id.clone()).one(db).await?.is_some();
    if !member_exists {
        let member = users::ActiveModel {
            id: ActiveValue::Set(member_id.clone()),
            email: ActiveValue::Set("member@example.com".to_string()),
            password_hash: ActiveValue::Set(hash_password("password")?),
            display_name: ActiveValue::Set("Regular Member".to_string()),
            global_role: ActiveValue::Set(GlobalUserRole::Member.to_string()),
            status: ActiveValue::Set(UserStatus::Active.to_string()),
            created_at: ActiveValue::Set(Utc::now()),
            updated_at: ActiveValue::Set(Utc::now()),
        };
        users::Entity::insert(member).exec(db).await?;
    }

    // 3. Create Default Workspace
    let ws_id = "ws_default".to_string();
    let ws_exists = workspaces::Entity::find_by_id(ws_id.clone()).one(db).await?.is_some();
    if !ws_exists {
        let workspace = workspaces::ActiveModel {
            id: ActiveValue::Set(ws_id.clone()),
            name: ActiveValue::Set("Default Workspace".to_string()),
            slug: ActiveValue::Set("default".to_string()),
            status: ActiveValue::Set(WorkspaceStatus::Active.to_string()),
            guest_join_enabled: ActiveValue::Set(true),
            token_ttl_seconds: ActiveValue::Set(3600),
            guest_access: ActiveValue::Set(GuestAccessPolicy::Allowed.to_string()),
            active_signing_secret_id: ActiveValue::NotSet,
            active_signing_secret_version: ActiveValue::NotSet,
            created_at: ActiveValue::Set(Utc::now()),
            updated_at: ActiveValue::Set(Utc::now()),
        };
        workspaces::Entity::insert(workspace).exec(db).await?;
    }

    // 4. Create Memberships
    let m1_id = format!("{}:{}", ws_id, admin_id);
    if workspace_memberships::Entity::find_by_id(m1_id.clone()).one(db).await?.is_none() {
        let membership = workspace_memberships::ActiveModel {
            id: ActiveValue::Set(m1_id),
            workspace_id: ActiveValue::Set(ws_id.clone()),
            user_id: ActiveValue::Set(admin_id),
            role: ActiveValue::Set(WorkspaceRole::Owner.to_string()),
            created_at: ActiveValue::Set(Utc::now()),
        };
        workspace_memberships::Entity::insert(membership).exec(db).await?;
    }

    let m2_id = format!("{}:{}", ws_id, member_id);
    if workspace_memberships::Entity::find_by_id(m2_id.clone()).one(db).await?.is_none() {
        let membership = workspace_memberships::ActiveModel {
            id: ActiveValue::Set(m2_id),
            workspace_id: ActiveValue::Set(ws_id.clone()),
            user_id: ActiveValue::Set(member_id),
            role: ActiveValue::Set(WorkspaceRole::Member.to_string()),
            created_at: ActiveValue::Set(Utc::now()),
        };
        workspace_memberships::Entity::insert(membership).exec(db).await?;
    }

    Ok(())
}
