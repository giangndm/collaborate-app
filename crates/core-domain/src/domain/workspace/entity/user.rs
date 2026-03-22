use crate::workspace::{DisplayName, GlobalUserRole, UserEmail, UserId, UserStatus};

/// Minimal profile data that belongs to the global user entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    email: UserEmail,
    display_name: DisplayName,
}

impl UserProfile {
    // TODO(task-4): Centralize profile normalization and validation here when
    // registration/edit flows exist, including email canonicalization and any
    // display-name policy that should be shared across entry points.
    pub fn new(email: UserEmail, display_name: DisplayName) -> Self {
        Self {
            email,
            display_name,
        }
    }

    pub fn email(&self) -> &UserEmail {
        &self.email
    }

    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }
}

/// Exists to model a person who can access the platform without mixing in any
/// workspace-scoped assignment details.
///
/// This struct represents the canonical global user record, including identity,
/// account lifecycle status, global role, and profile data.
///
/// Future developers should use this type whenever behavior depends on global
/// account state or profile information; use workspace membership entities for
/// workspace-specific roles instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    role: GlobalUserRole,
    status: UserStatus,
    profile: UserProfile,
}

impl User {
    // TODO(task-4): Add global account creation rules here once onboarding is
    // designed, such as which roles may be assigned directly and whether some
    // users must begin in a pending or invited state.
    pub fn new(id: UserId, role: GlobalUserRole, profile: UserProfile) -> Self {
        Self {
            id,
            role,
            status: UserStatus::Active,
            profile,
        }
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn role(&self) -> GlobalUserRole {
        self.role
    }

    pub fn status(&self) -> UserStatus {
        self.status
    }

    pub fn profile(&self) -> &UserProfile {
        &self.profile
    }

    // TODO(task-4): Guard re-activation with the final account recovery rules,
    // including checks for disabled accounts that require admin review before
    // they may become active again.
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
    }

    // TODO(task-4): Capture suspension-specific behavior here once account
    // governance rules exist, such as audit metadata and temporary login bans.
    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
    }

    // TODO(task-4): Replace this with the permanent deactivation workflow after
    // retention, recovery, and compliance rules for user accounts are defined.
    pub fn disable(&mut self) {
        self.status = UserStatus::Disabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{DisplayName, GlobalUserRole, UserEmail, UserId, UserStatus};

    #[test]
    fn user_construction_keeps_global_identity_role_and_profile() {
        let user = User::new(
            UserId::new("user_alpha"),
            GlobalUserRole::Member,
            UserProfile::new(
                UserEmail::new("alpha@example.com"),
                DisplayName::new("Alpha"),
            ),
        );

        assert_eq!(user.id(), &UserId::new("user_alpha"));
        assert_eq!(user.role(), GlobalUserRole::Member);
        assert_eq!(user.status(), UserStatus::Active);
        assert_eq!(user.profile().email().as_str(), "alpha@example.com");
        assert_eq!(user.profile().display_name().as_str(), "Alpha");
    }

    #[test]
    fn user_status_transitions_stay_small_and_obvious() {
        let mut user = User::new(
            UserId::new("user_beta"),
            GlobalUserRole::SuperAdmin,
            UserProfile::new(UserEmail::new("beta@example.com"), DisplayName::new("Beta")),
        );

        user.suspend();
        assert_eq!(user.status(), UserStatus::Suspended);

        user.disable();
        assert_eq!(user.status(), UserStatus::Disabled);

        user.activate();
        assert_eq!(user.status(), UserStatus::Active);
    }
}
