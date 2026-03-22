#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuestAccessPolicy {
    Denied,
    Allowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePolicy {
    pub guest_access: GuestAccessPolicy,
}

impl Default for WorkspacePolicy {
    fn default() -> Self {
        Self {
            guest_access: GuestAccessPolicy::Denied,
        }
    }
}
