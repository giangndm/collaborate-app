use std::collections::HashMap;

use automorph::Automorph;
use derive_more::{Display, FromStr};

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
}

#[derive(Debug, Default, Automorph)]
struct DocumentState {
    title: String,
    content: String,
}

pub struct DocumentApp {
    state: StateC<HashMap<DocumentId, DocumentState>, DocumentAppChannel>,
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
                self.state.insert(
                    id.clone(),
                    DocumentState {
                        title: String::new(),
                        content: String::new(),
                    },
                );
            }
            DocumentMutation::SetTitle(id, title) => {
                let doc = self
                    .state
                    .get_mut(&id)
                    .ok_or(DocumentError::DocumentNotFound)?;
                doc.title = title;
            }
            DocumentMutation::SetContent(id, content) => {
                let doc = self
                    .state
                    .get_mut(&id)
                    .ok_or(DocumentError::DocumentNotFound)?;
                doc.content = content;
            }
            DocumentMutation::Delete(id) => {
                self.state.remove(&id);
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
