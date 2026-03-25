use std::ops::Deref;

use rand::{Rng, distr::Alphanumeric, rng};
use syncable_state::{DeltaBatch, RuntimeState, SyncableState};

use crate::MemberInfo;

pub type SyncChange = DeltaBatch;

pub trait SyncableBlock {
    type Ctx;
    type Channel;
    type Mutation;
    type Error;

    /// Is this member allowed subscribe to provided channel?
    fn subscribe(&self, ctx: &Self::Ctx, member: &MemberInfo, channel: Self::Channel) -> bool;
    fn mutation(&mut self, ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error>;
    fn apply(&mut self, channel: Self::Channel, change: SyncChange);
    fn poll(&mut self) -> Option<(Self::Channel, SyncChange)>;
}

/***
 *
 * Wrap state with channel for pinning it with static channel
 *
 */

/// State wrap with channel
pub struct StateC<S: SyncableState + Clone, C> {
    o: State<S>,
    channel: C,
}

impl<S: SyncableState + Default + Clone, C: PartialEq + Clone> StateC<S, C> {
    pub fn new(channel: C) -> Self {
        Self {
            o: State::new(S::default()),
            channel,
        }
    }

    pub fn new_with_state(state: S, channel: C) -> Self {
        Self {
            o: State::new(state),
            channel,
        }
    }

    pub fn apply(&mut self, channel: C, change: SyncChange) {
        if channel != self.channel {
            panic!("Channel mismatch");
        }
        self.o.apply(change);
    }

    pub fn poll(&mut self) -> Option<(C, SyncChange)> {
        self.o.poll().map(|change| (self.channel.clone(), change))
    }
}

impl<S: SyncableState + Clone, C> Deref for StateC<S, C> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.o
    }
}

impl<S: SyncableState + Clone, C> std::ops::DerefMut for StateC<S, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.o
    }
}

/***
 * Logic for channel
 */

pub struct State<S: SyncableState + Clone> {
    runtime: RuntimeState<S>,
}

impl<S: SyncableState + Default + Clone> Default for State<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S: SyncableState + Clone> From<S> for State<S> {
    fn from(state: S) -> Self {
        Self::new(state)
    }
}

impl<S: SyncableState + Clone> State<S> {
    pub fn new(state: S) -> Self {
        Self::with_node_id(random_node_id(), state)
    }

    pub fn with_node_id(node_id: impl AsRef<str>, state: S) -> Self {
        Self {
            runtime: RuntimeState::new(node_id.as_ref(), state),
        }
    }

    pub fn apply(&mut self, change: SyncChange) {
        if let Err(e) = self.runtime.apply_remote(change) {
            log::error!("[State] Failed to apply remote delta: {}", e);
        }
    }

    pub fn poll(&mut self) -> Option<SyncChange> {
        self.runtime.poll()
    }
}

fn random_node_id() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

impl<S: SyncableState + Clone> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.runtime.state()
    }
}

impl<S: SyncableState + Clone> std::ops::DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime.state_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syncable_state::{SyncableCounter, SyncableState, SyncableString};

    #[derive(Debug, Clone, SyncableState)]
    struct TestState {
        #[sync(id)]
        pub id: String,
        pub s: SyncableString,
        pub v: SyncableCounter,
    }

    impl Default for TestState {
        fn default() -> Self {
            Self {
                id: "test".into(),
                s: SyncableString::default(),
                v: SyncableCounter::default(),
            }
        }
    }

    #[test_log::test]
    fn test_state_sync() {
        let mut state1 = State::new(TestState::default());
        let mut state2 = State::new(TestState::default());

        state1.v.increment(42).unwrap();
        state1.s.set("hello").unwrap();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.v.value(), state2.v.value());
        assert_eq!(state1.s.value(), state2.s.value());
    }

    #[test_log::test]
    fn test_state_sync_bootstraps_remote_state_without_conflict() {
        let mut state1 = State::with_node_id("00", TestState::default());
        let mut state2 = State::with_node_id("ff", TestState::default());

        state1.v.increment(42).unwrap();
        state1.s.set("hello").unwrap();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.v.value(), state2.v.value());
        assert_eq!(state1.s.value(), state2.s.value());
    }

    #[test_log::test]
    fn test_state_sync_with_random_node_id() {
        let mut state1 = State::new(TestState::default());
        let mut state2 = State::new(TestState::default());

        state1.v.increment(7).unwrap();
        state1.s.set("random").unwrap();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.v.value(), state2.v.value());
        assert_eq!(state1.s.value(), state2.s.value());
    }

    #[test_log::test]
    fn test_poll_only_emits_unsent_changes() {
        let mut sender = State::with_node_id("sender", TestState::default());

        sender.v.increment(1).unwrap();
        let first_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
        assert!(!first_batch.is_empty());

        sender.v.increment(1).unwrap();
        let second_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
        assert!(!second_batch.is_empty());

        assert!(sender.poll().is_none());
    }
}
