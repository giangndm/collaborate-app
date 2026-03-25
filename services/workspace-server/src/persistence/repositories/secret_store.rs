use async_trait::async_trait;
use core_domain::workspace::{
    SecretStore, WorkspaceApiKeyId, WorkspaceApiKeyMetadata, WorkspaceApiKeySecret,
    WorkspaceCredentialStatus, WorkspaceError, WorkspaceId, WorkspaceLastUpdated, WorkspaceResult,
    WorkspaceSecretRef, WorkspaceSecretRefId, WorkspaceSecretVersion,
};
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::str::FromStr;

#[derive(Clone)]
pub struct SqliteSecretStore {
    db: DatabaseConnection,
}

impl SqliteSecretStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SecretStore for SqliteSecretStore {
    async fn list_api_keys(
        &self,
        workspace_id: &WorkspaceId,
    ) -> WorkspaceResult<Vec<WorkspaceApiKeyMetadata>> {
        use crate::persistence::entities::{
            workspace_credential_secret_versions, workspace_credentials,
        };

        let credential_models = workspace_credentials::Entity::find()
            .filter(workspace_credentials::Column::WorkspaceId.eq(workspace_id.to_string()))
            .all(&self.db)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        let mut results = Vec::new();
        for cred in credential_models {
            let latest_version = workspace_credential_secret_versions::Entity::find()
                .filter(
                    workspace_credential_secret_versions::Column::ApiKeyId
                        .eq(cred.api_key_id.clone()),
                )
                .order_by_desc(workspace_credential_secret_versions::Column::Version)
                .one(&self.db)
                .await
                .map_err(|e| WorkspaceError::Internal(e.to_string()))?
                .ok_or_else(|| {
                    WorkspaceError::Internal(format!("No secret versions for {}", cred.api_key_id))
                })?;

            results.push(WorkspaceApiKeyMetadata {
                api_key_id: WorkspaceApiKeyId::new(cred.api_key_id),
                label: cred.label,
                secret_ref: WorkspaceSecretRef {
                    secret_ref_id: WorkspaceSecretRefId::new(latest_version.id.to_string()),
                    version: WorkspaceSecretVersion::new(latest_version.version as u64),
                },
                status: WorkspaceCredentialStatus::from_str(&cred.status)
                    .map_err(|e| WorkspaceError::Internal(e.to_string()))?,
                created_at: WorkspaceLastUpdated::from_rfc3339(&cred.created_at.to_rfc3339())
                    .map_err(|e| WorkspaceError::Internal(e.to_string()))?,
                rotated_at: None, // TODO: Store rotated_at in DB if needed
            });
        }

        Ok(results)
    }

    async fn create_api_key(
        &self,
        workspace_id: &WorkspaceId,
        label: &str,
    ) -> WorkspaceResult<WorkspaceApiKeySecret> {
        use crate::persistence::entities::{
            workspace_credential_secret_versions, workspace_credentials,
        };
        use uuid::Uuid;

        let db = &self.db;
        let txn = db
            .begin()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        let api_key_id = format!("ak_{}", Uuid::new_v4().to_string().replace("-", ""));
        let api_secret = format!("sk_{}", Uuid::new_v4().to_string().replace("-", ""));

        let cred_active = workspace_credentials::ActiveModel {
            api_key_id: ActiveValue::Set(api_key_id.clone()),
            workspace_id: ActiveValue::Set(workspace_id.to_string()),
            label: ActiveValue::Set(label.to_string()),
            status: ActiveValue::Set(WorkspaceCredentialStatus::Active.to_string()),
            created_at: ActiveValue::Set(chrono::Utc::now()),
        };

        workspace_credentials::Entity::insert(cred_active)
            .exec(&txn)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        let version_active = workspace_credential_secret_versions::ActiveModel {
            api_key_id: ActiveValue::Set(api_key_id.clone()),
            secret_hash: ActiveValue::Set(api_secret.clone()), // Plain strictly for the skeleton
            version: ActiveValue::Set(1),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        workspace_credential_secret_versions::Entity::insert(version_active)
            .exec(&txn)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(WorkspaceApiKeySecret {
            api_key_id: WorkspaceApiKeyId::new(api_key_id),
            api_secret,
            status: WorkspaceCredentialStatus::Active,
            version: WorkspaceSecretVersion::new(1),
        })
    }

    async fn rotate_api_key_secret(
        &self,
        workspace_id: &WorkspaceId,
        api_key_id: &WorkspaceApiKeyId,
    ) -> WorkspaceResult<WorkspaceApiKeySecret> {
        use crate::persistence::entities::{
            workspace_credential_secret_versions, workspace_credentials,
        };
        use uuid::Uuid;

        let db = &self.db;
        let txn = db
            .begin()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        let cred = workspace_credentials::Entity::find_by_id(api_key_id.to_string())
            .filter(workspace_credentials::Column::WorkspaceId.eq(workspace_id.to_string()))
            .one(&txn)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?
            .ok_or_else(|| WorkspaceError::Internal("API Key not found".to_string()))?;

        let latest_version = workspace_credential_secret_versions::Entity::find()
            .filter(
                workspace_credential_secret_versions::Column::ApiKeyId.eq(api_key_id.to_string()),
            )
            .order_by_desc(workspace_credential_secret_versions::Column::Version)
            .one(&txn)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?
            .ok_or_else(|| WorkspaceError::Internal("No versions found".to_string()))?;

        let new_secret = format!("sk_rot_{}", Uuid::new_v4().to_string().replace("-", ""));
        let new_version = latest_version.version + 1;

        let version_active = workspace_credential_secret_versions::ActiveModel {
            api_key_id: ActiveValue::Set(api_key_id.to_string()),
            secret_hash: ActiveValue::Set(new_secret.clone()),
            version: ActiveValue::Set(new_version),
            created_at: ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        workspace_credential_secret_versions::Entity::insert(version_active)
            .exec(&txn)
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        let status = WorkspaceCredentialStatus::from_str(&cred.status)
            .map_err(|e| WorkspaceError::Internal(e.to_string()))?;

        Ok(WorkspaceApiKeySecret {
            api_key_id: api_key_id.clone(),
            api_secret: new_secret,
            status,
            version: WorkspaceSecretVersion::new(new_version as u64),
        })
    }
}
