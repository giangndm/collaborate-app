use core::fmt::Display;
use core::str::FromStr;
use std::collections::BTreeMap;

use crate::{
    ApplyChildPath, ApplyPath, ChangeEnvelope, ChangeOp, FieldSchema, MapOp, PathSegment,
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
/// # let item = Item { id: "item-1".into(), value: SyncableString::from("hello") };
/// let mut map: SyncableMap<String, Item> = SyncableMap::default();
/// let mut runtime = RuntimeState::new("node-A", map);
///
/// runtime.with_batch(|state, batch| {
///     state.insert(batch, "item-1".into(), item.clone())?;
///     Ok::<(), syncable_state::SyncError>(())
/// }).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncableMap<K, V> {
    root_path: SyncPath,
    tracker: Option<crate::EventTracker>,
    entries: BTreeMap<K, V>,
}

impl<K, V> SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    pub(crate) fn new(root_path: SyncPath) -> Self {
        Self {
            root_path,
            tracker: None,
            entries: BTreeMap::new(),
        }
    }

    pub fn from_entries<I>(entries: I) -> Result<Self, SyncError>
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut value = Self::default();
        for (key, entry) in entries {
            let key_str = key.to_string();
            let snapshot = V::snapshot_to_value(entry.snapshot());
            let decoded = value.decode_value(&key_str, snapshot)?;
            if value.entries.insert(key.clone(), decoded).is_some() {
                return Err(SyncError::DuplicateMapKey { key: key_str });
            }
        }
        Ok(value)
    }

    pub fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<(), SyncError> {
        if self.entries.contains_key(&key) {
            return Err(SyncError::InvalidPath);
        }

        let key_str = key.to_string();
        let snapshot = V::snapshot_to_value(value.snapshot());
        validate_snapshot_value_for::<V>(&V::schema(), &snapshot)?;
        let mut value = match self.decode_value(&key_str, snapshot.clone()) {
            Ok(value) => value,
            Err(error) => {
                return Err(error);
            }
        };

        let mut child_path = self.root_path.clone().into_vec();
        child_path.push(PathSegment::Key(key_str.clone()));
        value.rebind_paths(SyncPath::new(child_path), self.tracker.clone());

        self.entries.insert(key.clone(), value);
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::Map(MapOp::Insert {
                    key: key_str,
                    value: snapshot,
                }),
            ));
        }
        Ok(())
    }

    pub fn replace(&mut self, key: K, value: V) -> Result<(), SyncError> {
        if !self.entries.contains_key(&key) {
            return Err(SyncError::InvalidPath);
        }

        let key_str = key.to_string();
        let snapshot = V::snapshot_to_value(value.snapshot());
        validate_snapshot_value_for::<V>(&V::schema(), &snapshot)?;
        let mut value = match self.decode_value(&key_str, snapshot.clone()) {
            Ok(value) => value,
            Err(error) => {
                return Err(error);
            }
        };

        let mut child_path = self.root_path.clone().into_vec();
        child_path.push(PathSegment::Key(key_str.clone()));
        value.rebind_paths(SyncPath::new(child_path), self.tracker.clone());

        self.entries.insert(key.clone(), value);
        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::Map(MapOp::Replace {
                    key: key_str,
                    value: snapshot,
                }),
            ));
        }
        Ok(())
    }

    pub fn remove(&mut self, key: &K) -> Result<(), SyncError> {
        match self.entries.remove(key) {
            Some(_) => {}
            None => {
                return Err(SyncError::InvalidPath);
            }
        }

        if let Some(tracker) = &self.tracker {
            tracker.borrow_mut().push(ChangeEnvelope::new(
                self.root_path.clone(),
                ChangeOp::Map(MapOp::Remove {
                    key: key.to_string(),
                }),
            ));
        }
        Ok(())
    }

    fn decode_value(&self, key: &str, value: crate::SnapshotValue) -> Result<V, SyncError> {
        let mut child_root = self.root_path.clone().into_vec();
        child_root.push(PathSegment::Key(key.to_string()));
        <V as SnapshotCodec>::snapshot_from_value(SyncPath::new(child_root), value)
    }
}

impl<K, V> SyncContainer for SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = BTreeMap<String, V::Snapshot>;

    fn root_path(&self) -> &SyncPath {
        &self.root_path
    }

    fn snapshot_value(&self) -> Self::Snapshot {
        self.entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.snapshot()))
            .collect()
    }

    fn apply_path_tail(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        match (path, op) {
            ([], ChangeOp::Map(MapOp::Insert { key, value })) => {
                let k = K::from_str(key).map_err(|_| SyncError::InvalidPath)?;
                if self.entries.contains_key(&k) {
                    return Err(SyncError::InvalidPath);
                }
                validate_snapshot_value_for::<V>(&V::schema(), value)?;
                let decoded = self.decode_value(key, value.clone())?;
                self.entries.insert(k, decoded);
                Ok(())
            }
            ([], ChangeOp::Map(MapOp::Replace { key, value })) => {
                let k = K::from_str(key).map_err(|_| SyncError::InvalidPath)?;
                if !self.entries.contains_key(&k) {
                    return Err(SyncError::InvalidPath);
                }
                validate_snapshot_value_for::<V>(&V::schema(), value)?;
                let decoded = self.decode_value(key, value.clone())?;
                self.entries.insert(k, decoded);
                Ok(())
            }
            ([], ChangeOp::Map(MapOp::Remove { key })) => {
                let k = K::from_str(key).map_err(|_| SyncError::InvalidPath)?;
                if self.entries.remove(&k).is_none() {
                    return Err(SyncError::InvalidPath);
                }
                Ok(())
            }
            ([PathSegment::Key(key), rest @ ..], _) => {
                let k = K::from_str(key).map_err(|_| SyncError::InvalidPath)?;
                self.entries
                    .get_mut(&k)
                    .ok_or(SyncError::InvalidPath)?
                    .apply_child_path(rest, op)
            }
            _ => Err(SyncError::InvalidPath),
        }
    }
}

impl<K, V> ApplyPath for SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn apply_path(&mut self, path: &[PathSegment], op: &ChangeOp) -> Result<(), SyncError> {
        if let Some(tail) = path.strip_prefix(self.root_path.as_slice()) {
            return self.apply_path_tail(tail, op);
        }

        Err(SyncError::InvalidPath)
    }
}

impl<K, V> SyncableState for SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    type Snapshot = BTreeMap<String, V::Snapshot>;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_value()
    }

    fn rebind_paths(&mut self, root_path: SyncPath, tracker: Option<crate::EventTracker>) {
        self.root_path = root_path;
        self.tracker = tracker.clone();

        for (key, value) in &mut self.entries {
            let mut child_root = self.root_path.clone().into_vec();
            child_root.push(PathSegment::Key(key.to_string()));
            value.rebind_paths(SyncPath::new(child_root), tracker.clone());
        }
    }

    fn schema() -> StateSchema {
        StateSchema::new(vec![FieldSchema {
            name: "root".into(),
            kind: crate::FieldKind::Map,
        }])
    }
}

impl<K, V> SnapshotCodec for SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
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
            let k = K::from_str(&key).map_err(|_| SyncError::InvalidSnapshotValue)?;
            let mut child_root = root_path.clone().into_vec();
            child_root.push(PathSegment::Key(key.clone()));
            let entry = V::snapshot_from_value(SyncPath::new(child_root), entry_value)?;
            if decoded.entries.insert(k, entry).is_some() {
                return Err(SyncError::DuplicateMapKey { key });
            }
        }

        Ok(decoded)
    }
}

impl<K, V> Default for SyncableMap<K, V>
where
    K: Clone + Ord + Display + FromStr,
    V: SyncableState + SnapshotCodec + ApplyChildPath + 'static,
{
    fn default() -> Self {
        Self::new(SyncPath::default())
    }
}
