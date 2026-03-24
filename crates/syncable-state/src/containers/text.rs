use crate::{
    ApplyPath, BatchTx, ChangeEnvelope, ChangeOp, FieldSchema, PathSegment, SnapshotCodec,
    SnapshotValue, StateSchema, SyncContainer, SyncError, SyncPath, SyncableState, TextContainer,
    TextOp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableText {
    root_path: SyncPath,
    value: String,
}

impl SyncableText {
    pub fn new(root_path: SyncPath, value: impl Into<String>) -> Self {
        Self {
            root_path,
            value: value.into(),
        }
    }

    pub fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn splice(
        &mut self,
        batch: &mut BatchTx<'_>,
        index: usize,
        delete: usize,
        insert: impl Into<String>,
    ) -> Result<(), SyncError> {
        TextContainer::splice(self, batch, index, delete, insert.into())
    }

    pub fn clear(&mut self, batch: &mut BatchTx<'_>) -> Result<(), SyncError> {
        TextContainer::clear(self, batch)
    }

    fn apply_op(&mut self, op: &TextOp) -> Result<(), SyncError> {
        match op {
            TextOp::Splice {
                index,
                delete,
                insert,
            } => self.apply_splice(*index, *delete, insert),
            TextOp::Clear => {
                self.value.clear();
                Ok(())
            }
        }
    }

    fn apply_splice(
        &mut self,
        index: usize,
        delete_len: usize,
        insert: &str,
    ) -> Result<(), SyncError> {
        let end = index
            .checked_add(delete_len)
            .ok_or(SyncError::InvalidTextRange)?;
        if end > self.value.len()
            || !self.value.is_char_boundary(index)
            || !self.value.is_char_boundary(end)
        {
            return Err(SyncError::InvalidTextRange);
        }

        self.value.replace_range(index..end, insert);
        Ok(())
    }
}

impl SyncContainer for SyncableText {
    type Snapshot = String;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.value.clone()
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::Text(op)) => self.apply_op(op),
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl TextContainer for SyncableText {
    fn value(&self) -> &str {
        &self.value
    }

    fn splice(
        &mut self,
        batch: &mut BatchTx<'_>,
        index: usize,
        delete: usize,
        insert: String,
    ) -> Result<(), SyncError> {
        if let Err(error) = self.apply_splice(index, delete, &insert) {
            batch.poison();
            return Err(error);
        }
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::Text(TextOp::Splice {
                index,
                delete,
                insert,
            }),
        ));
        Ok(())
    }

    fn clear(&mut self, batch: &mut BatchTx<'_>) -> Result<(), SyncError> {
        self.value.clear();
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::Text(TextOp::Clear),
        ));
        Ok(())
    }
}

impl ApplyPath for SyncableText {
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl SyncableState for SyncableText {
    type Snapshot = String;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_value()
    }

    fn rebind_paths(&mut self, root_path: SyncPath) {
        self.root_path = root_path;
    }

    fn is_scalar_value() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![FieldSchema {
            name: "root".into(),
            kind: crate::FieldKind::Text,
        }])
    }
}

impl SnapshotCodec for SyncableText {
    fn snapshot_to_value(snapshot: Self::Snapshot) -> SnapshotValue {
        SnapshotValue::String(snapshot)
    }

    fn snapshot_from_value(root_path: SyncPath, value: SnapshotValue) -> Result<Self, SyncError> {
        match value {
            SnapshotValue::String(value) => Ok(Self::new(root_path, value)),
            _ => Err(SyncError::InvalidSnapshotValue),
        }
    }
}
