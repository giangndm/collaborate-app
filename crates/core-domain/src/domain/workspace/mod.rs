mod entity;
mod guards;
mod ports;
mod service;
pub mod types;

pub use entity::*;
pub use guards::*;
pub use ports::*;
pub use service::*;
pub use types::*;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_ports_module_stays_trait_only() {
        let ports_module = include_str!("ports/mod.rs");

        assert!(!ports_module.contains("#[cfg(test)]"));
        assert!(!ports_module.contains("struct "));
        assert!(!ports_module.contains("impl "));
        assert!(!ports_module.contains("fn sample_"));
    }

    #[test]
    fn workspace_port_contracts_stay_narrow_and_documented() {
        let user_repository = include_str!("ports/user_repository.rs");
        let workspace_repository = include_str!("ports/workspace_repository.rs");
        let membership_repository = include_str!("ports/membership_repository.rs");
        let secret_store = include_str!("ports/secret_store.rs");

        assert!(!user_repository.contains("fn save"));
        assert!(user_repository.contains("Returns an error when the user is absent."));

        assert!(workspace_repository.contains("Returns an error when the workspace is absent."));
        assert!(workspace_repository.contains("Saves the full aggregate state as an upsert."));

        assert!(membership_repository.contains("Returns an error when the membership is absent."));
        assert!(membership_repository.contains(
            "Returns an error when the membership is absent for that workspace-user pair."
        ));
        assert!(membership_repository.contains(
            "Returns memberships in storage-defined order; callers must not rely on sorting."
        ));
        assert!(membership_repository.contains("Saves the membership as an upsert."));

        assert!(secret_store.contains(
            "Returns API-key metadata in storage-defined order; callers must not rely on sorting."
        ));
        assert!(!secret_store.contains("TODO"));
    }
}
