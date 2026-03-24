use derive_more::{Display, FromStr};
use syncable_state::{
    PathSegment, SyncError, SyncPath, SyncableMap, SyncableState, SyncableString,
};

use crate::{
    apps::AppCtx,
    sync::{StateC, SyncChange, SyncableBlock},
};

pub enum DocumentMutation {
    Create(DocumentId),
    SetTitle(DocumentId, String),
    SetContent(DocumentId, String),
    Delete(DocumentId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentAppChannel {
    Public,
}

#[derive(Debug, Clone, FromStr, Display, Eq, Hash, PartialEq)]
pub struct DocumentId(String);

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("Document not found")]
    DocumentNotFound,
    #[error("Sync error: {0}")]
    SyncError(#[from] SyncError),
}

#[derive(Debug, Clone, SyncableState)]
struct DocumentState {
    #[sync(id)]
    pub id: String,
    pub title: SyncableString,
    pub content: SyncableString,
}

impl DocumentState {
    pub fn new(root_path: &SyncPath, id: String) -> Self {
        let mut path_title = root_path.clone().into_vec();
        path_title.push(PathSegment::Key(id.clone()));
        path_title.push(PathSegment::Field("title".into()));

        let mut path_content = root_path.clone().into_vec();
        path_content.push(PathSegment::Key(id.clone()));
        path_content.push(PathSegment::Field("content".into()));

        Self {
            id,
            title: SyncableString::new(SyncPath::new(path_title), ""),
            content: SyncableString::new(SyncPath::new(path_content), ""),
        }
    }
}

#[derive(Debug, Clone, SyncableState)]
struct DocumentAppState {
    docs: SyncableMap<DocumentState>,
}

impl Default for DocumentAppState {
    fn default() -> Self {
        Self {
            docs: SyncableMap::new(SyncPath::from_field("docs")),
        }
    }
}

pub struct DocumentApp {
    state: StateC<DocumentAppState, DocumentAppChannel>,
}

impl DocumentApp {
    pub fn new() -> Self {
        Self {
            state: StateC::new(DocumentAppChannel::Public),
        }
    }
}

impl SyncableBlock for DocumentApp {
    type Ctx = AppCtx;

    type Channel = DocumentAppChannel;
    type Mutation = DocumentMutation;

    type Error = DocumentError;

    fn subscribe(
        &self,
        _ctx: &Self::Ctx,
        _member: &crate::MemberInfo,
        _channel: Self::Channel,
    ) -> bool {
        true
    }

    fn mutation(&mut self, _ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error> {
        match mutation {
            DocumentMutation::Create(id) => {
                let id_str = id.to_string();
                self.state.mutate(|state, batch| {
                    state.docs.insert(
                        batch,
                        id_str.clone(),
                        DocumentState::new(state.docs.root_path(), id_str),
                    )?;
                    Ok::<(), SyncError>(())
                })?;
            }
            DocumentMutation::SetTitle(id, title) => {
                let id_str = id.to_string();
                let mut found = true;
                self.state.mutate(|state, batch| {
                    if let Some(doc) = state.docs.get_mut(&id_str) {
                        doc.title.set(batch, title)?;
                    } else {
                        found = false;
                    }
                    Ok::<(), SyncError>(())
                })?;
                if !found {
                    return Err(DocumentError::DocumentNotFound);
                }
            }
            DocumentMutation::SetContent(id, content) => {
                let id_str = id.to_string();
                let mut found = true;
                self.state.mutate(|state, batch| {
                    if let Some(doc) = state.docs.get_mut(&id_str) {
                        doc.content.set(batch, content)?;
                    } else {
                        found = false;
                    }
                    Ok::<(), SyncError>(())
                })?;
                if !found {
                    return Err(DocumentError::DocumentNotFound);
                }
            }
            DocumentMutation::Delete(id) => {
                let id_str = id.to_string();
                self.state.mutate(|state, batch| {
                    state.docs.remove(batch, &id_str)?;
                    Ok::<(), SyncError>(())
                })?;
            }
        }
        Ok(())
    }

    fn apply(&mut self, channel: Self::Channel, change: SyncChange) {
        self.state.apply(channel, change);
    }

    fn poll(&mut self) -> Option<(Self::Channel, SyncChange)> {
        self.state.poll()
    }
}
