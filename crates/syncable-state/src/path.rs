use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReplicaId(String);

impl core::fmt::Display for ReplicaId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ReplicaId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ReplicaId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ReplicaId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PathSegment {
    Field(String),
    Key(String),
    Id(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SyncPath(Vec<PathSegment>);

impl SyncPath {
    pub fn new(segments: Vec<PathSegment>) -> Self {
        Self(segments)
    }

    pub fn from_field(name: impl Into<String>) -> Self {
        Self(vec![PathSegment::Field(name.into())])
    }

    pub fn as_slice(&self) -> &[PathSegment] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<PathSegment> {
        self.0
    }
}
