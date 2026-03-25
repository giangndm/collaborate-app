use crate::{SnapshotValue, SyncPath};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeEnvelope {
    pub path: SyncPath,
    pub op: ChangeOp,
}

impl ChangeEnvelope {
    pub fn new(path: SyncPath, op: ChangeOp) -> Self {
        Self { path, op }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeOp {
    String(StringOp),
    Counter(CounterOp),
    Text(TextOp),
    List(ListOp),
    Map(MapOp),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StringOp {
    Set(String),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CounterOp {
    Increment(i64),
    Decrement(i64),
    Multiply(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextOp {
    Splice {
        index: usize,
        delete: usize,
        insert: String,
    },
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ListOp {
    Insert {
        id: String,
        after: Option<String>,
        value: SnapshotValue,
    },
    Delete {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MapOp {
    Insert { key: String, value: SnapshotValue },
    Replace { key: String, value: SnapshotValue },
    Remove { key: String },
}
