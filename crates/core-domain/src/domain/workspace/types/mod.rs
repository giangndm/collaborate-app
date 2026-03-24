//! Strongly typed workspace-domain contracts live here.

mod credentials;
mod errors;
mod ids;
mod membership;
mod permissions;
mod policy;
mod status;
mod sync;
mod user;

pub use credentials::*;
pub use errors::*;
pub use ids::*;
pub use membership::*;
pub use permissions::*;
pub use policy::*;
pub use status::*;
pub use sync::*;
pub use user::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ids_keep_workspace_and_user_boundaries_clear() {
        let workspace_id = WorkspaceId::from("ws_123".to_owned());
        let user_id = UserId::from("user_456".to_owned());

        assert_eq!(workspace_id.to_string(), "ws_123");
        assert_eq!(user_id.to_string(), "user_456");
        assert_ne!(workspace_id.to_string(), user_id.to_string());
    }

    #[test]
    fn baseline_roles_and_workspace_permissions_are_explicit() {
        let workspace_id = WorkspaceId::from("ws_core".to_owned());
        let read_permission = WorkspaceReadPermission::new(workspace_id.clone());
        let write_permission = WorkspaceWritePermission::new(workspace_id.clone());

        assert_ne!(GlobalUserRole::Member, GlobalUserRole::SuperAdmin);
        assert_ne!(WorkspaceRole::Owner, WorkspaceRole::Member);
        assert_eq!(read_permission.workspace_id(), &workspace_id);
        assert_eq!(write_permission.workspace_id(), &workspace_id);
    }

    #[test]
    fn workspace_error_formats_typed_context_for_tracing() {
        let error = WorkspaceError::PermissionDenied {
            user_id: UserId::from("user_789".to_owned()),
            workspace_id: WorkspaceId::from("ws_trace".to_owned()),
        };

        assert_eq!(
            error.to_string(),
            "permission denied for user user_789 on workspace ws_trace"
        );
    }

    #[test]
    fn workspace_permission_mismatch_error_formats_both_workspace_ids() {
        let error = WorkspaceError::WorkspacePermissionMismatch {
            permission_workspace_id: WorkspaceId::from("ws_permission".to_owned()),
            target_workspace_id: WorkspaceId::from("ws_target".to_owned()),
        };

        assert_eq!(
            error.to_string(),
            "workspace permission mismatch: permission for ws_permission cannot write target ws_target"
        );
    }

    #[test]
    fn workspace_last_updated_round_trips_as_rfc3339_timestamp() {
        let last_updated = WorkspaceLastUpdated::from_rfc3339("2026-03-23T10:00:00Z")
            .expect("rfc3339 timestamp should parse");

        assert_eq!(last_updated.to_rfc3339(), "2026-03-23T10:00:00.000Z");
        assert!(last_updated > WorkspaceLastUpdated::initial());
    }
}
