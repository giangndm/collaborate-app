use crate::{
    containers::validate_snapshot_value_for, ApplyChildPath, ApplyPath, BatchTx, ChangeEnvelope,
    ChangeOp, FieldSchema, ListOp, PathSegment, SnapshotCodec, StableId, StateSchema,
    SyncContainer, SyncError, SyncPath, SyncableState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableVec<T> {
    root_path: SyncPath,
    items: Vec<T>,
}

impl<T> SyncableVec<T>
where
    T: StableId + SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    pub fn new(root_path: SyncPath) -> Self {
        Self {
            root_path,
            items: Vec::new(),
        }
    }

    pub fn from_items(root_path: SyncPath, items: Vec<T>) -> Result<Self, SyncError> {
        let mut value = Self::new(root_path);
        for item in items {
            let id = item.stable_id().to_string();
            let snapshot = T::snapshot_to_value(item.snapshot());
            let item = value.decode_item(&id, snapshot)?;
            let after = value
                .items
                .last()
                .map(|existing| existing.stable_id().to_string());
            value.insert_item_after(item, after.as_deref())?;
        }
        Ok(value)
    }

    pub fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    pub fn get(&self, id: &str) -> Option<&T> {
        self.items.iter().find(|item| item.stable_id() == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut T> {
        self.items.iter_mut().find(|item| item.stable_id() == id)
    }

    pub fn insert(&mut self, batch: &mut BatchTx<'_>, item: T) -> Result<(), SyncError> {
        let id = item.stable_id().to_string();
        let after = self
            .items
            .last()
            .map(|existing| existing.stable_id().to_string());
        let snapshot = T::snapshot_to_value(item.snapshot());
        if let Err(error) = validate_snapshot_value_for::<T>(&T::schema(), &snapshot) {
            batch.poison();
            return Err(error);
        }
        let item = match self.decode_item(&id, snapshot.clone()) {
            Ok(item) => item,
            Err(error) => {
                batch.poison();
                return Err(error);
            }
        };
        if let Err(error) = self.insert_item_after(item, after.as_deref()) {
            batch.poison();
            return Err(error);
        }
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::List(ListOp::Insert {
                id,
                after,
                value: snapshot,
            }),
        ));
        Ok(())
    }

    pub fn delete(&mut self, batch: &mut BatchTx<'_>, id: &str) -> Result<(), SyncError> {
        if let Err(error) = self.remove_by_id(id) {
            batch.poison();
            return Err(error);
        }
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::List(ListOp::Delete { id: id.into() }),
        ));
        Ok(())
    }

    fn decode_item(&self, id: &str, value: crate::SnapshotValue) -> Result<T, SyncError> {
        let mut child_root = self.root_path.clone().into_vec();
        child_root.push(PathSegment::Id(id.to_string()));
        let item = <T as SnapshotCodec>::snapshot_from_value(SyncPath::new(child_root), value)?;
        if item.stable_id() != id {
            return Err(SyncError::InvalidSnapshotValue);
        }
        Ok(item)
    }

    fn insert_item_after(&mut self, item: T, after: Option<&str>) -> Result<(), SyncError> {
        let id = item.stable_id().to_string();
        if self.get(&id).is_some() {
            return Err(SyncError::DuplicateStableId { id });
        }

        let index = match after {
            Some(after_id) => {
                let Some(position) = self
                    .items
                    .iter()
                    .position(|existing| existing.stable_id() == after_id)
                else {
                    return Err(SyncError::StableIdNotFound {
                        id: after_id.to_string(),
                    });
                };
                position + 1
            }
            None => 0,
        };
        self.items.insert(index, item);
        Ok(())
    }

    fn remove_by_id(&mut self, id: &str) -> Result<(), SyncError> {
        let Some(index) = self.items.iter().position(|item| item.stable_id() == id) else {
            return Err(SyncError::StableIdNotFound { id: id.into() });
        };
        self.items.remove(index);
        Ok(())
    }
}

impl<T> SyncContainer for SyncableVec<T>
where
    T: StableId + SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = Vec<T::Snapshot>;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.items.iter().map(SyncableState::snapshot).collect()
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::List(ListOp::Insert { id, after, value })) => {
                validate_snapshot_value_for::<T>(&T::schema(), value)?;
                let item = self.decode_item(id, value.clone())?;
                self.insert_item_after(item, after.as_deref())
            }
            ([], ChangeOp::List(ListOp::Delete { id })) => self.remove_by_id(id),
            ([PathSegment::Id(id), rest @ ..], _) => self
                .get_mut(id)
                .ok_or_else(|| SyncError::StableIdNotFound { id: id.clone() })?
                .apply_child_path(rest, op),
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl<T> ApplyPath for SyncableVec<T>
where
    T: StableId + SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl<T> SyncableState for SyncableVec<T>
where
    T: StableId + SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = Vec<T::Snapshot>;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_value()
    }

    fn rebind_paths(&mut self, root_path: SyncPath) {
        self.root_path = root_path;

        for item in &mut self.items {
            let mut child_root = self.root_path.clone().into_vec();
            child_root.push(PathSegment::Id(item.stable_id().to_string()));
            item.rebind_paths(SyncPath::new(child_root));
        }
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![FieldSchema {
            name: "root".into(),
            kind: crate::FieldKind::List,
        }])
    }
}

impl<T> SnapshotCodec for SyncableVec<T>
where
    T: StableId + SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn snapshot_to_value(snapshot: Self::Snapshot) -> crate::SnapshotValue {
        crate::SnapshotValue::List(
            snapshot
                .into_iter()
                .map(T::snapshot_to_value)
                .collect::<Vec<_>>(),
        )
    }

    fn snapshot_from_value(
        root_path: SyncPath,
        value: crate::SnapshotValue,
    ) -> Result<Self, SyncError> {
        let crate::SnapshotValue::List(items) = value else {
            return Err(SyncError::InvalidSnapshotValue);
        };

        let mut decoded = Self::new(root_path.clone());
        let mut after: Option<String> = None;

        for item_value in items {
            let mut item = T::snapshot_from_value(root_path.clone(), item_value)?;
            let id = item.stable_id().to_string();

            let mut child_root = root_path.clone().into_vec();
            child_root.push(PathSegment::Id(id.clone()));
            item.rebind_paths(SyncPath::new(child_root));

            decoded.insert_item_after(item, after.as_deref())?;
            after = Some(id);
        }

        Ok(decoded)
    }
}
