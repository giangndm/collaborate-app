#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceRole {
    Owner,
    Admin,
    Member,
}
