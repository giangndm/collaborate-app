use derive_more::Display;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum WorkspaceStatus {
    #[display("active")]
    Active,
    #[display("suspended")]
    Suspended,
    #[display("disabled")]
    Disabled,
}

impl FromStr for WorkspaceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "disabled" => Ok(Self::Disabled),
            _ => Err(format!("invalid workspace status: {s}")),
        }
    }
}
