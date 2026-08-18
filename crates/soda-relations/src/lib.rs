use soda_ast::{Clause, ClauseDetail, Module, Substitution, Term, match_term};
use soda_semantics::{Config, Engine, Status};
use soda_sorts::TypeChecker;
use soda_verifier::validate_module;
use std::collections::{BTreeMap, BTreeSet};

mod search;
mod verification;
pub const MAX_DERIVATION_DEPTH: usize = 128;
pub const DEFAULT_SEARCH_DEPTH: usize = 64;
pub const DEFAULT_MAX_ANSWERS: usize = 100;

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub max_depth: usize,
    pub max_attempts: usize,
    pub max_answers: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: DEFAULT_SEARCH_DEPTH,
            max_attempts: soda_semantics::DEFAULT_FUEL,
            max_answers: DEFAULT_MAX_ANSWERS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Derivation {
    pub rule: String,
    pub conclusion: Term,
    pub substitution: Substitution,
    pub premises: Vec<Derivation>,
}

#[derive(Clone, Debug)]
pub struct Answer {
    pub term: Term,
    pub bindings: Substitution,
    pub derivation: Derivation,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub answers: Vec<Answer>,
    /// True only if every branch was exhausted without hitting any limit.
    pub complete: bool,
    pub attempts: usize,
}

#[derive(Clone, Debug)]
pub struct Relations {
    clauses: Vec<Clause>,
    variables: Vec<BTreeSet<String>>,
    clause_indices: BTreeMap<String, usize>,
    details: BTreeMap<String, ClauseDetail>,
    conditions: Engine,
    functions: BTreeSet<String>,
    schema: TypeChecker,
}

impl Relations {
    pub fn new(clauses: Vec<Clause>) -> Result<Self, String> {
        let module = Module {
            clauses,
            ..Module::default()
        };

        Self::from_module(&module)
    }

    pub fn from_module(module: &Module) -> Result<Self, String> {
        validate_module(module).map_err(|error| error.to_string())?;
        soda_sorts::validate_module(module).map_err(|error| error.to_string())?;

        Ok(Self {
            clauses: module.clauses.clone(),
            variables: module
                .clauses
                .iter()
                .map(|clause| {
                    let mut variables = clause_variables(clause);
                    if let Some(detail) = module
                        .clause_details
                        .iter()
                        .find(|detail| detail.clause == clause.name)
                    {
                        for binding in &detail.bindings {
                            variables.extend(binding.pattern.variables());
                            variables.extend(binding.value.variables());
                        }
                    }
                    variables
                })
                .collect(),
            clause_indices: module
                .clauses
                .iter()
                .enumerate()
                .map(|(index, clause)| (clause.name.clone(), index))
                .collect(),
            details: module
                .clause_details
                .iter()
                .map(|detail| (detail.clause.clone(), detail.clone()))
                .collect(),
            conditions: Engine::new(module.rules.clone()).map_err(|error| error.to_string())?,
            functions: module
                .signatures
                .iter()
                .filter(|signature| signature.kind == soda_ast::SignatureKind::Function)
                .map(|signature| signature.name.clone())
                .collect(),
            schema: TypeChecker::new(module).map_err(|error| error.to_string())?,
        })
    }

}

#[derive(Clone, Copy)]
enum Condition {
    True,
    False,
    Unknown,
}

fn clause_variables(clause: &Clause) -> BTreeSet<String> {
    let mut variables = clause.conclusion.variables();

    for premise in &clause.premises {
        variables.extend(premise.variables());
    }

    variables
}

struct Search<'a> {
    relations: &'a Relations,
    config: &'a SearchConfig,
    attempts: usize,
    complete: bool,
    fresh: usize,
    used: BTreeSet<String>,
}

