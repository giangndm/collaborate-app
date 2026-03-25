use serde::{Deserialize, Serialize};
use derive_more::{Display, FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, FromStr)]
pub enum WorkspaceRole {
    Owner,
    Member,
}
