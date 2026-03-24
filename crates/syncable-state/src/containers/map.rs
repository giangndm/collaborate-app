use std::collections::BTreeMap;

use crate::{
    ApplyChildPath, ApplyPath, BatchTx, ChangeEnvelope, ChangeOp, FieldSchema, MapOp, PathSegment,
    SnapshotCodec, StateSchema, SyncContainer, SyncError, SyncPath, SyncableState,
    containers::validate_snapshot_value_for,
};

/// A synchronization container that maps strings to child `SyncableState` elements.
///
/// `SyncableMap` allows dynamically storing, mutating, and replicating a variable
/// number of sub-properties. Since entries inside a `SyncableMap` must also implement
/// `SyncableState`, they can contain arbitrarily deep nested syncable structures.
///
/// # Example
///
/// ```rust
/// # use syncable_state::{SyncableState, SyncableMap, SyncPath, SyncableString, RuntimeState};
/// # #[derive(SyncableState, Clone)]
/// # pub struct Item {
/// #     #[sync(id)] id: String,
/// #     value: SyncableString,
/// # }
/// # let item = Item { id: "item-1".into(), value: SyncableString::new(SyncPath::from_field("v"), "hello") };
/// let mut map: SyncableMap<Item> = SyncableMap::new(SyncPath::from_field("items"));
/// let mut runtime = RuntimeState::new("node-A", map);
///
/// runtime.with_batch(|state, batch| {
///     state.insert(batch, "item-1", item.clone())?;
///     Ok::<(), syncable_state::SyncError>(())
/// }).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableMap<V> {
    root_path: SyncPath,
    entries: BTreeMap<String, V>,
}

impl<V> SyncableMap<V>
where
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    pub fn new(root_path: SyncPath) -> Self {
        Self {
            root_path,
            entries: BTreeMap::new(),
        }
    }

    pub fn from_entries<I>(root_path: SyncPath, entries: I) -> Result<Self, SyncError>
    where
        I: IntoIterator<Item = (String, V)>,
    {
        let mut value = Self::new(root_path);
        for (key, entry) in entries {
            let snapshot = V::snapshot_to_value(entry.snapshot());
            let decoded = value.decode_value(&key, snapshot)?;
            if value.entries.insert(key.clone(), decoded).is_some() {
                return Err(SyncError::DuplicateMapKey { key });
            }
        }
        Ok(value)
    }

    pub fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.entries.get_mut(key)
    }

    pub fn insert(
        &mut self,
        batch: &mut BatchTx<'_>,
        key: impl Into<String>,
        value: V,
    ) -> Result<(), SyncError> {
        let key = key.into();
        if self.entries.contains_key(&key) {
            batch.poison();
            return Err(SyncError::InvalidPath);
        }

        let snapshot = V::snapshot_to_value(value.snapshot());
        if let Err(error) = validate_snapshot_value_for::<V>(&V::schema(), &snapshot) {
            batch.poison();
            return Err(error);
        }
        let value = match self.decode_value(&key, snapshot.clone()) {
            Ok(value) => value,
            Err(error) => {
                batch.poison();
                return Err(error);
            }
        };
        self.entries.insert(key.clone(), value);
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::Map(MapOp::Insert {
                key,
                value: snapshot,
            }),
        ));
        Ok(())
    }

    pub fn replace(
        &mut self,
        batch: &mut BatchTx<'_>,
        key: impl Into<String>,
        value: V,
    ) -> Result<(), SyncError> {
        let key = key.into();
        if !self.entries.contains_key(&key) {
            batch.poison();
            return Err(SyncError::InvalidPath);
        }

        let snapshot = V::snapshot_to_value(value.snapshot());
        if let Err(error) = validate_snapshot_value_for::<V>(&V::schema(), &snapshot) {
            batch.poison();
            return Err(error);
        }
        let value = match self.decode_value(&key, snapshot.clone()) {
            Ok(value) => value,
            Err(error) => {
                batch.poison();
                return Err(error);
            }
        };
        self.entries.insert(key.clone(), value);
        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::Map(MapOp::Replace {
                key,
                value: snapshot,
            }),
        ));
        Ok(())
    }

    pub fn remove(&mut self, batch: &mut BatchTx<'_>, key: &str) -> Result<(), SyncError> {
        match self.entries.remove(key) {
            Some(_) => {}
            None => {
                batch.poison();
                return Err(SyncError::InvalidPath);
            }
        }

        batch.push(ChangeEnvelope::new(
            self.root_path.clone(),
            ChangeOp::Map(MapOp::Remove { key: key.into() }),
        ));
        Ok(())
    }

    fn decode_value(&self, key: &str, value: crate::SnapshotValue) -> Result<V, SyncError> {
        let mut child_root = self.root_path.clone().into_vec();
        child_root.push(PathSegment::Key(key.to_string()));
        <V as SnapshotCodec>::snapshot_from_value(SyncPath::new(child_root), value)
    }
}

impl<V> SyncContainer for SyncableMap<V>
where
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = BTreeMap<String, V::Snapshot>;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.entries
            .iter()
            .map(|(key, value)| (key.clone(), value.snapshot()))
            .collect()
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::Map(MapOp::Insert { key, value })) => {
                if self.entries.contains_key(key) {
                    return Err(SyncError::InvalidPath);
                }
                validate_snapshot_value_for::<V>(&V::schema(), value)?;
                let decoded = self.decode_value(key, value.clone())?;
                self.entries.insert(key.clone(), decoded);
                Ok(())
            }
            ([], ChangeOp::Map(MapOp::Replace { key, value })) => {
                if !self.entries.contains_key(key) {
                    return Err(SyncError::InvalidPath);
                }
                validate_snapshot_value_for::<V>(&V::schema(), value)?;
                let decoded = self.decode_value(key, value.clone())?;
                self.entries.insert(key.clone(), decoded);
                Ok(())
            }
            ([], ChangeOp::Map(MapOp::Remove { key })) => {
                if self.entries.remove(key).is_none() {
                    return Err(SyncError::InvalidPath);
                }
                Ok(())
            }
            ([PathSegment::Key(key), rest @ ..], _) => self
                .entries
                .get_mut(key)
                .ok_or(SyncError::InvalidPath)?
                .apply_child_path(rest, op),
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl<V> ApplyPath for SyncableMap<V>
where
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl<V> SyncableState for SyncableMap<V>
where
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = BTreeMap<String, V::Snapshot>;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_value()
    }

    fn rebind_paths(&mut self, root_path: SyncPath) {
        self.root_path = root_path;

        for (key, value) in &mut self.entries {
            let mut child_root = self.root_path.clone().into_vec();
            child_root.push(PathSegment::Key(key.clone()));
            value.rebind_paths(SyncPath::new(child_root));
        }
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![FieldSchema {
            name: "root".into(),
            kind: crate::FieldKind::Map,
        }])
    }
}

impl<V> SnapshotCodec for SyncableMap<V>
where
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn snapshot_to_value(snapshot: Self::Snapshot) -> crate::SnapshotValue {
        crate::SnapshotValue::Map(
            snapshot
                .into_iter()
                .map(|(key, value)| (key, V::snapshot_to_value(value)))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    fn snapshot_from_value(
        root_path: SyncPath,
        value: crate::SnapshotValue,
    ) -> Result<Self, SyncError> {
        let crate::SnapshotValue::Map(entries) = value else {
            return Err(SyncError::InvalidSnapshotValue);
        };

        let mut decoded = Self::new(root_path.clone());
        for (key, entry_value) in entries {
            let mut child_root = root_path.clone().into_vec();
            child_root.push(PathSegment::Key(key.clone()));
            let entry = V::snapshot_from_value(SyncPath::new(child_root), entry_value)?;
            if decoded.entries.insert(key.clone(), entry).is_some() {
                return Err(SyncError::DuplicateMapKey { key });
            }
        }

        Ok(decoded)
    }
}
