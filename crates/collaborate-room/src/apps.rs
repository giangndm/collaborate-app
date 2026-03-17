use crate::{
    SyncableBlock,
    apps::document::{DocumentApp, DocumentAppChange, DocumentError, DocumentMutation},
};

mod document;

pub struct AppCtx {}

pub enum AppRuntimeChange {
    Document(DocumentAppChange),
}

pub enum AppRuntimeMutation {
    Document(DocumentMutation),
}

#[derive(Debug, thiserror::Error)]
pub enum AppRuntimeError {
    #[error("document error: {0}")]
    Document(#[from] DocumentError),
}

pub struct AppRuntime {
    ctx: AppCtx,
    document: DocumentApp,
}

impl AppRuntime {
    pub fn new() -> Self {
        Self {
            ctx: AppCtx {},
            document: DocumentApp::new(),
        }
    }
}

impl SyncableBlock for AppRuntime {
    type Ctx = ();

    type Change = AppRuntimeChange;
    type Mutation = AppRuntimeMutation;
    type Error = AppRuntimeError;

    fn mutation(&mut self, _ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error> {
        match mutation {
            AppRuntimeMutation::Document(doc_mutation) => {
                self.document.mutation(&self.ctx, doc_mutation)?;
                Ok(())
            }
        }
    }

    fn apply(&mut self, change: Self::Change) {
        match change {
            AppRuntimeChange::Document(change) => {
                self.document.apply(change);
            }
        }
    }

    fn poll(&mut self) -> Option<Self::Change> {
        if let Some(out) = self.document.poll() {
            return Some(AppRuntimeChange::Document(out));
        }

        None
    }
}
