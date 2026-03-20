use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

use automerge::{ActorId, AutoCommit, Change, ChangeHash, ROOT, ReadDoc};
use automorph::{Automorph, ChangeReport};
use rand::{Rng, distr::Alphanumeric, rng};

use crate::MemberInfo;

pub type SyncChange = Change;

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
pub struct StateC<S, C> {
    o: State<S>,
    channel: C,
}

impl<S: Automorph + Default, C: PartialEq + Clone> StateC<S, C> {
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

    pub fn apply(&mut self, channel: C, change: Change) {
        if channel != self.channel {
            panic!("Channel mismatch");
        }
        self.o.apply(change);
    }

    pub fn poll(&mut self) -> Option<(C, Change)> {
        self.o.poll().map(|change| (self.channel.clone(), change))
    }
}

impl<S, C> Deref for StateC<S, C> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.o
    }
}

impl<S, C> DerefMut for StateC<S, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.o
    }
}

/***
 * Logic for channel
 */

#[derive(Debug)]
pub struct State<S> {
    state: S,
    doc: AutoCommit,
    changes: VecDeque<Change>,
    sent_heads: Vec<ChangeHash>,
}

impl<S: Automorph + Default> Default for State<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S: Automorph> From<S> for State<S> {
    fn from(state: S) -> Self {
        Self::new(state)
    }
}

impl<S: Automorph> State<S> {
    pub fn new(state: S) -> Self {
        Self::with_node_id(random_node_id(), state)
    }

    pub fn with_node_id(node_id: impl AsRef<str>, state: S) -> Self {
        Self {
            state,
            doc: doc_with_node_id(node_id.as_ref()),
            changes: VecDeque::new(),
            sent_heads: Vec::new(),
        }
    }

    pub fn apply(&mut self, change: Change) {
        self.doc.apply_changes([change]).unwrap();
        self.sent_heads = self.doc.get_heads();
        if let Err(e) = self.state.update(&self.doc, ROOT, "state") {
            log::error!("[State] Failed to update state: {}", e);
        }
    }

    pub fn poll(&mut self) -> Option<Change> {
        if let Some(change) = self.pop_pending_change() {
            return Some(change);
        }

        self.publish_if_needed()?;
        self.pop_pending_change()
    }

    fn pop_pending_change(&mut self) -> Option<Change> {
        self.changes.pop_front()
    }

    fn publish_if_needed(&mut self) -> Option<()> {
        if !self.needs_publish()? {
            return None;
        }

        self.save_state()?;
        self.queue_new_changes();
        Some(())
    }

    fn needs_publish(&self) -> Option<bool> {
        if !self.state_exists()? {
            return Some(true);
        }

        self.state
            .diff(&self.doc, ROOT, "state")
            .map(|diff| !diff.none())
            .ok()
    }

    fn state_exists(&self) -> Option<bool> {
        self.doc
            .get(ROOT, "state")
            .map(|state| state.is_some())
            .ok()
    }

    fn save_state(&mut self) -> Option<()> {
        self.state
            .save(&mut self.doc, ROOT, "state")
            .map_err(|e| {
                log::error!("[State] Failed to save state: {}", e);
            })
            .ok()
    }

    fn queue_new_changes(&mut self) {
        self.changes.extend(self.doc.get_changes(&self.sent_heads));
        self.sent_heads = self.doc.get_heads();
    }
}

fn doc_with_node_id(node_id: &str) -> AutoCommit {
    AutoCommit::new().with_actor(ActorId::from(node_id.as_bytes()))
}

fn random_node_id() -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect()
}

impl<S> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Automorph, PartialEq, Eq)]
    struct TestState {
        s: String,
        v: u32,
    }

    #[test_log::test]
    fn test_state_sync() {
        let mut state1 = State::new(TestState::default());
        let mut state2 = State::new(TestState::default());

        state1.v = 42;
        state1.s = "hello".to_string();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.deref(), state2.deref());
    }

    #[test_log::test]
    fn test_state_sync_bootstraps_remote_state_without_conflict() {
        let mut state1 = State::with_node_id("00", TestState::default());
        let mut state2 = State::with_node_id("ff", TestState::default());

        state1.v = 42;
        state1.s = "hello".to_string();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.deref(), state2.deref());
    }

    #[test_log::test]
    fn test_state_sync_with_random_node_id() {
        let mut state1 = State::new(TestState::default());
        let mut state2 = State::new(TestState::default());

        state1.v = 7;
        state1.s = "random".to_string();

        while let Some(change) = state1.poll() {
            state2.apply(change);
        }

        assert_eq!(state1.deref(), state2.deref());
    }

    #[test_log::test]
    fn test_poll_only_emits_unsent_changes() {
        let mut sender = State::with_node_id("sender", TestState::default());

        sender.v = 1;
        let first_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
        assert!(!first_batch.is_empty());
        let frontier_after_first_publish = sender.doc.get_heads();

        sender.v = 2;
        let second_batch: Vec<_> = std::iter::from_fn(|| sender.poll()).collect();
        assert!(!second_batch.is_empty());
        let expected_delta = sender.doc.get_changes(&frontier_after_first_publish);

        assert_eq!(second_batch.len(), expected_delta.len());
        assert_eq!(second_batch, expected_delta);
    }

    #[test_log::test]
    fn test_poll_does_not_replay_remote_applied_history() {
        let mut original = State::with_node_id("original", TestState::default());
        let mut relay = State::with_node_id("relay", TestState::default());

        original.v = 1;
        for change in std::iter::from_fn(|| original.poll()) {
            relay.apply(change);
        }
        let frontier_after_import = relay.doc.get_heads();

        relay.s = "local".to_string();
        let forwarded: Vec<_> = std::iter::from_fn(|| relay.poll()).collect();
        let expected_delta = relay.doc.get_changes(&frontier_after_import);

        assert_eq!(forwarded.len(), expected_delta.len());
        assert_eq!(forwarded, expected_delta);
    }
}
