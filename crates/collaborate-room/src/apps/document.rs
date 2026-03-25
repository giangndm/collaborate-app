use derive_more::{Display, FromStr};
use syncable_state::{SyncError, SyncableMap, SyncableState, SyncableString};

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

#[derive(Debug, Clone, FromStr, Display, Eq, Hash, PartialEq, Ord, PartialOrd)]
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
    pub id: DocumentId,
    pub title: SyncableString,
    pub content: SyncableString,
}

impl DocumentState {
    pub fn new(id: DocumentId) -> Self {
        Self {
            id,
            title: SyncableString::default(),
            content: SyncableString::default(),
        }
    }
}

#[derive(Debug, Clone, SyncableState, Default)]
struct DocumentAppState {
    docs: SyncableMap<DocumentId, DocumentState>,
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
                self.state
                    .docs
                    .insert(id.clone(), DocumentState::new(id.clone()))?;
            }
            DocumentMutation::SetTitle(id, title) => {
                let mut found = true;
                if let Some(doc) = self.state.docs.get_mut(&id) {
                    doc.title.set(title)?;
                } else {
                    found = false;
                }
                if !found {
                    return Err(DocumentError::DocumentNotFound);
                }
            }
            DocumentMutation::SetContent(id, content) => {
                let mut found = true;
                if let Some(doc) = self.state.docs.get_mut(&id) {
                    doc.content.set(content)?;
                } else {
                    found = false;
                }
                if !found {
                    return Err(DocumentError::DocumentNotFound);
                }
            }
            DocumentMutation::Delete(id) => {
                self.state.docs.remove(&id)?;
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
