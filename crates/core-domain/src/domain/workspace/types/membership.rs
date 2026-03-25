use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
pub enum WorkspaceRole {
    Owner,
    Member,
}
