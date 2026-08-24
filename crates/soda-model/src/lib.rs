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

    fn mark_incomplete(&mut self, state: usize, reason: Incomplete) {
        self.states[state].status = StateStatus::Incomplete;

        if !self.incompleteness.contains(&reason) {
            self.incompleteness.push(reason);
        }
    }
}

/// Explores in BFS order. Budgets limit new states; edges to known states are retained.
pub fn explore(engine: &Engine, initial: &Term, config: &Config) -> Exploration {
    let mut graph = Exploration {
        states: Vec::new(),
        transitions: Vec::new(),
        incompleteness: Vec::new(),
        indices: BTreeMap::new(),
        parents: Vec::new(),
    };

    if config.max_states == 0 {
        graph
            .incompleteness
            .push(Incomplete::StateLimit { state: None });

        return graph;
    }

    graph.indices.insert(initial.clone(), 0);
    graph.parents.push(None);
    graph.states.push(State {
        term: initial.clone(),
        depth: 0,
        status: StateStatus::Expanded,
    });
    let mut queue = VecDeque::from([0]);

    while let Some(from) = queue.pop_front() {
        let source = graph.states[from].term.clone();
        let successors = match engine.successors(&source, &config.evaluation) {
            Ok(steps) => steps,
            Err(status) => {
                graph.mark_incomplete(
                    from,
                    Incomplete::Enumeration {
                        state: from,
                        status,
                    },
                );
                continue;
            }
        };

        if successors.is_empty() {
            graph.states[from].status = StateStatus::Terminal;
        }

        for step in successors {
            let target = if source.at(&step.path) == Some(&step.before) {
                source.replace(&step.path, step.after)
            } else {
                None
            };

            let Some(target) = target else {
                graph.mark_incomplete(
                    from,
                    Incomplete::InvalidTransition {
                        state: from,
                        path: step.path,
                    },
                );
                continue;
            };

            let to = if let Some(&existing) = graph.indices.get(&target) {
                existing
            } else {
                if graph.states[from].depth >= config.max_depth {
                    graph.mark_incomplete(from, Incomplete::DepthLimit { state: from });
                    continue;
                }

                if graph.states.len() >= config.max_states {
                    graph.mark_incomplete(from, Incomplete::StateLimit { state: Some(from) });
                    continue;
                }

                let index = graph.states.len();
                graph.indices.insert(target.clone(), index);
                graph.states.push(State {
                    term: target,
                    depth: graph.states[from].depth + 1,
                    status: StateStatus::Expanded,
                });
                graph.parents.push(Some(graph.transitions.len()));
                queue.push_back(index);

                index
            };

            graph.transitions.push(Transition {
                from,
                to,
                rule: step.rule,
                path: step.path,
            });
        }
    }

    graph
}
