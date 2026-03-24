use syncable_state::{SyncError, SyncPath, SyncableMap, SyncableState};

use crate::{
    apps::{AppRuntime, AppRuntimeChannel, AppRuntimeError, AppRuntimeMutation},
    sync::{StateC, SyncChange, SyncableBlock},
    types::{MemberId, MemberInfo},
};

pub enum RoomMutation {
    AddMember(MemberInfo),
    RemoveMember(MemberId),
    AppMutation(AppRuntimeMutation),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoomChannel {
    Room,
    App(AppRuntimeChannel),
}

#[derive(Debug, thiserror::Error)]
pub enum RoomError {
    #[error("Member not found")]
    MemberNotFound,
    #[error("App runtime error: {0}")]
    AppRuntime(#[from] AppRuntimeError),
    #[error("Sync error: {0}")]
    SyncError(#[from] SyncError),
}

#[derive(Debug, Clone, SyncableState)]
struct RoomState {
    members: SyncableMap<MemberInfo>,
}

impl Default for RoomState {
    fn default() -> Self {
        Self {
            members: SyncableMap::new(SyncPath::from_field("members")),
        }
    }
}

pub struct CollaborateRoom {
    state: StateC<RoomState, RoomChannel>,
    app_runtime: AppRuntime,
}

impl Default for CollaborateRoom {
    fn default() -> Self {
        Self::new()
    }
}

impl CollaborateRoom {
    pub fn new() -> Self {
        Self {
            state: StateC::new(RoomChannel::Room),
            app_runtime: AppRuntime::new(),
        }
    }
}

impl SyncableBlock for CollaborateRoom {
    type Channel = RoomChannel;
    type Mutation = RoomMutation;
    type Error = RoomError;
    type Ctx = ();

    fn subscribe(&self, _ctx: &Self::Ctx, member: &MemberInfo, channel: Self::Channel) -> bool {
        match channel {
            RoomChannel::Room => true,
            RoomChannel::App(channel) => self.app_runtime.subscribe(_ctx, member, channel),
        }
    }

    fn mutation(&mut self, ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error> {
        match mutation {
            RoomMutation::AddMember(member_info) => {
                let id = member_info.id.clone();
                self.state.mutate(|state, batch| {
                    state.members.insert(batch, id.as_str(), member_info)?;
                    Ok::<(), SyncError>(())
                })?;
                Ok(())
            }
            RoomMutation::RemoveMember(member_id) => {
                self.state.mutate(|state, batch| {
                    state.members.remove(batch, member_id.as_str())?;
                    Ok::<(), SyncError>(())
                })?;
                Ok(())
            }
            RoomMutation::AppMutation(app_mutation) => {
                self.app_runtime.mutation(ctx, app_mutation)?;
                Ok(())
            }
        }
    }

    fn apply(&mut self, channel: Self::Channel, change: SyncChange) {
        match channel {
            RoomChannel::Room => {
                self.state.apply(channel, change);
            }
            RoomChannel::App(channel) => {
                self.app_runtime.apply(channel, change);
            }
        }
    }

    fn poll(&mut self) -> Option<(Self::Channel, SyncChange)> {
        if let Some((channel, change)) = self.state.poll() {
            return Some((channel, change));
        }
        if let Some((channel, change)) = self.app_runtime.poll() {
            return Some((RoomChannel::App(channel), change));
        }
        None
    }
}
