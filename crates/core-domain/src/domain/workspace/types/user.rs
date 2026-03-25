use derive_more::{Display, From, TryFrom};
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
pub enum GlobalUserRole {
    #[display("member")]
    Member,
    #[display("super_admin")]
    SuperAdmin,
}

impl FromStr for GlobalUserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "member" => Ok(Self::Member),
            "super_admin" => Ok(Self::SuperAdmin),
            _ => Err(format!("invalid global user role: {s}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum UserStatus {
    #[display("active")]
    Active,
    #[display("suspended")]
    Suspended,
    #[display("disabled")]
    Disabled,
}

impl FromStr for UserStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "disabled" => Ok(Self::Disabled),
            _ => Err(format!("invalid user status: {s}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Serialize, Deserialize)]
pub struct UserEmail(String);

impl UserEmail {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display, From, Serialize, Deserialize)]
pub struct DisplayName(String);

impl DisplayName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
