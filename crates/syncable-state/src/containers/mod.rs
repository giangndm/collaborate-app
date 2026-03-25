mod counter;
mod map;
mod string;
mod text;
mod vec;

pub use counter::SyncableCounter;
pub use map::SyncableMap;
pub use string::SyncableString;
pub use text::SyncableText;
pub use vec::SyncableVec;

use crate::{FieldKind, SnapshotValue, StateSchema, SyncError, SyncableState};

pub(crate) fn validate_snapshot_value_for<T>(
    schema: &StateSchema,
    value: &SnapshotValue,
) -> Result<(), SyncError>
where
    T: SyncableState,
{
    if schema.fields.is_empty() {
        return Ok(());
    }

    if T::is_scalar_value() {
        return validate_kind(&schema.fields[0].kind, value);
    }

    let SnapshotValue::Map(fields) = value else {
        return Err(SyncError::InvalidSnapshotValue);
    };

    for field in &schema.fields {
        let Some(field_value) = fields.get(&field.name) else {
            return Err(SyncError::InvalidSnapshotValue);
        };
        validate_kind(&field.kind, field_value)?;
    }

    if fields.len() != schema.fields.len() {
        return Err(SyncError::InvalidSnapshotValue);
    }

    Ok(())
}

fn validate_kind(kind: &FieldKind, value: &SnapshotValue) -> Result<(), SyncError> {
    match (kind, value) {
        (FieldKind::String | FieldKind::Text, SnapshotValue::String(_)) => Ok(()),
        (FieldKind::Counter, SnapshotValue::Counter(_)) => Ok(()),
        (FieldKind::List, SnapshotValue::List(_)) => Ok(()),
        (FieldKind::Map | FieldKind::Object, SnapshotValue::Map(_)) => Ok(()),
        _ => Err(SyncError::InvalidSnapshotValue),
    }
}
