use derive_more::{Display, From};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlobalUserRole {
    Member,
    SuperAdmin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserStatus {
    Active,
    Suspended,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From)]
pub struct UserEmail(String);

impl UserEmail {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
