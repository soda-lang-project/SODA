//! Incomplete exploration can find counterexamples, but cannot establish safety.
use soda_ast::Term;
use soda_semantics::{Engine, Status};
use std::collections::{BTreeMap, VecDeque};

pub const DEFAULT_MAX_STATES: usize = 10_000;
pub const DEFAULT_EXPLORATION_DEPTH: usize = 100;

/// Graph budgets are independent from each transition's evaluation budgets.
#[derive(Clone, Debug)]
pub struct Config {
    /// Maximum stored states, including the initial state. Zero explores none.
    pub max_states: usize,
    /// Maximum shortest-path distance for newly discovered states.
    pub max_depth: usize,
    pub evaluation: soda_semantics::Config,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_states: DEFAULT_MAX_STATES,
            max_depth: DEFAULT_EXPLORATION_DEPTH,
            evaluation: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateStatus {
    /// Every discovered transition is stored, and there is at least one.
    Expanded,
    /// Successor enumeration succeeded with no transitions.
    Terminal,
    /// Enumeration failed or at least one successor exceeded a graph budget.
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub term: Term,
    pub depth: usize,
    pub status: StateStatus,
}

/// Distinct rule applications and paths remain separate edges, even with equal targets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub from: usize,
    pub to: usize,
    pub rule: String,
    pub path: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Incomplete {
    /// `None` denotes a zero state budget that excluded the initial state.
    StateLimit {
        state: Option<usize>,
    },
    DepthLimit {
        state: usize,
    },
    Enumeration {
        state: usize,
        status: Status,
    },
    InvalidTransition {
        state: usize,
        path: Vec<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reachability {
    Reachable {
        state: usize,
        witness: Vec<Transition>,
    },
    Unreachable,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvariantResult {
    Holds,
    Violated {
        state: usize,
        witness: Vec<Transition>,
    },
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Exploration {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
    pub incompleteness: Vec<Incomplete>,
    indices: BTreeMap<Term, usize>,
    /// Index of the first transition that discovered each state (BFS tree).
    parents: Vec<Option<usize>>,
}

impl Exploration {
    pub fn is_complete(&self) -> bool {
        self.incompleteness.is_empty()
    }

    /// Returns a shortest path, empty for the root; `None` means undiscovered.
    pub fn witness(&self, target: &Term) -> Option<Vec<Transition>> {
        self.indices
            .get(target)
            .map(|&state| self.witness_to(state))
    }

    fn witness_to(&self, mut state: usize) -> Vec<Transition> {
        let mut path = Vec::new();

        while let Some(index) = self.parents[state] {
            let edge = &self.transitions[index];
            path.push(edge.clone());
            state = edge.from;
        }

        path.reverse();

        path
    }

    pub fn reachability(&self, target: &Term) -> Reachability {
        match self.indices.get(target) {
            Some(&state) => Reachability::Reachable {
                state,
                witness: self.witness_to(state),
            },
            None if self.is_complete() => Reachability::Unreachable,
            None => Reachability::Unknown,
        }
    }

    /// Checks stored states, returning a shortest witness even when exploration is incomplete.
    pub fn check_invariant(&self, predicate: impl Fn(&Term) -> bool) -> InvariantResult {
        for (state, value) in self.states.iter().enumerate() {
            if !predicate(&value.term) {
                return InvariantResult::Violated {
                    state,
                    witness: self.witness_to(state),
                };
            }
        }

        if self.is_complete() {
            InvariantResult::Holds
        } else {
            InvariantResult::Unknown
        }
    }

}

