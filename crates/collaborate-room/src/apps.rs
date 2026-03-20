use crate::{
    MemberInfo,
    apps::document::{DocumentApp, DocumentAppChannel, DocumentError, DocumentMutation},
    sync::{SyncChange, SyncableBlock},
};

mod document;

pub struct AppCtx {}

#[derive(Debug, Clone, PartialEq)]
pub enum AppRuntimeChannel {
    Document(DocumentAppChannel),
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

    type Channel = AppRuntimeChannel;
    type Mutation = AppRuntimeMutation;
    type Error = AppRuntimeError;

    fn subscribe(&self, _ctx: &Self::Ctx, member: &MemberInfo, channel: Self::Channel) -> bool {
        match channel {
            AppRuntimeChannel::Document(channel) => {
                self.document.subscribe(&self.ctx, member, channel)
            }
        }
    }

    fn mutation(&mut self, _ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error> {
        match mutation {
            AppRuntimeMutation::Document(doc_mutation) => {
                self.document.mutation(&self.ctx, doc_mutation)?;
                Ok(())
            }
        }
    }

    fn apply(&mut self, channel: Self::Channel, change: SyncChange) {
        match channel {
            AppRuntimeChannel::Document(channel) => {
                self.document.apply(channel, change);
            }
        }
    }

    fn poll(&mut self) -> Option<(Self::Channel, SyncChange)> {
        if let Some((channel, change)) = self.document.poll() {
            return Some((AppRuntimeChannel::Document(channel), change));
        }

        None
    }
}
