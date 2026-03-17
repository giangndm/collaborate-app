use std::collections::HashMap;

use crate::{
    State, SyncableBlock,
    apps::{AppRuntime, AppRuntimeChange},
    types::{MemberId, MemberInfo},
};

use automerge::Change;
use automorph::Automorph;

pub enum RoomMutation {
    AddMember(MemberInfo),
    RemoveMember(MemberId),
}

pub enum RoomChange {
    Room(Change),
    App(AppRuntimeChange),
}

#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Member not found")]
    MemberNotFound,
}

#[derive(Debug, Default, Automorph)]
struct RoomState {
    members: HashMap<MemberId, MemberInfo>,
}

pub struct CollaborateRoom {
    state: State<RoomState>,
    app_runtime: AppRuntime,
}

impl CollaborateRoom {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
            app_runtime: AppRuntime::new(),
        }
    }
}

impl SyncableBlock for CollaborateRoom {
    type Change = RoomChange;
    type Mutation = RoomMutation;
    type Error = RoomError;
    type Ctx = ();

    fn mutation(&mut self, ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error> {
        match mutation {
            RoomMutation::AddMember(member_info) => {
                self.state
                    .members
                    .insert(member_info.id.clone(), member_info);
                Ok(())
            }
            RoomMutation::RemoveMember(member_id) => {
                self.state.members.remove(&member_id);
                Ok(())
            }
        }
    }

    fn apply(&mut self, change: Self::Change) {
        match change {
            RoomChange::Room(change) => {
                self.state.apply(change);
            }
            RoomChange::App(change) => {
                self.app_runtime.apply(change);
            }
        }
    }

    fn poll(&mut self) -> Option<Self::Change> {
        if let Some(change) = self.state.poll() {
            return Some(RoomChange::Room(change));
        }
        if let Some(change) = self.app_runtime.poll() {
            return Some(RoomChange::App(change));
        }
        None
    }
}
