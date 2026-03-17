use std::collections::HashMap;

use automerge::Change;
use automorph::Automorph;
use derive_more::{Display, FromStr};

use crate::{State, SyncableBlock, apps::AppCtx};

pub enum DocumentMutation {
    Create(DocumentId),
    SetTitle(DocumentId, String),
    SetContent(DocumentId, String),
    Delete(DocumentId),
}

pub type DocumentAppChange = Change;

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
    state: State<HashMap<DocumentId, DocumentState>>,
}

impl DocumentApp {
    pub fn new() -> Self {
        Self {
            state: HashMap::new().into(),
        }
    }
}

impl SyncableBlock for DocumentApp {
    type Ctx = AppCtx;

    type Change = DocumentAppChange;

    type Mutation = DocumentMutation;

    type Error = DocumentError;

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

    fn apply(&mut self, change: Self::Change) {
        self.state.apply(change);
    }

    fn poll(&mut self) -> Option<Self::Change> {
        todo!()
    }
}
