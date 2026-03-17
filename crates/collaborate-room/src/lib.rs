use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
};

use automerge::{AutoCommit, Change, ROOT};
use automorph::{Automorph, ChangeReport};

mod apps;
mod room;
mod types;

pub trait SyncableBlock {
    type Ctx;
    type Change;

    type Mutation;
    type Error;

    fn mutation(&mut self, ctx: &Self::Ctx, mutation: Self::Mutation) -> Result<(), Self::Error>;
    fn apply(&mut self, change: Self::Change);
    fn poll(&mut self) -> Option<Self::Change>;
}

#[derive(Debug)]
pub struct State<S> {
    state: S,
    doc: AutoCommit,
    changes: VecDeque<Change>,
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
        let mut doc = AutoCommit::new();
        if let Err(e) = state.save(&mut doc, ROOT, "state") {
            log::error!("[State] Failed to save state: {}", e);
        }
        Self {
            state,
            doc,
            changes: VecDeque::new(),
        }
    }

    pub fn apply(&mut self, change: Change) {
        self.doc.apply_changes([change]).unwrap();
        if let Err(e) = self.state.update(&self.doc, ROOT, "state") {
            log::error!("[State] Failed to update state: {}", e);
        }
    }

    pub fn poll(&mut self) -> Option<Change> {
        if let Some(change) = self.changes.pop_front() {
            return Some(change);
        }

        match self.state.diff(&self.doc, ROOT, "state") {
            Ok(diff) => {
                if diff.none() {
                    return None;
                }
            }
            Err(_) => {
                return None;
            }
        }

        if let Err(e) = self.state.save(&mut self.doc, ROOT, "state") {
            log::error!("[State] Failed to save state: {}", e);
            return None;
        }
        self.changes.extend(self.doc.get_changes(&[]));
        self.changes.pop_front()
    }
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
            println!("change: {change:?}");
            state2.apply(change);
        }

        assert_eq!(state1.deref(), state2.deref());
    }
}
