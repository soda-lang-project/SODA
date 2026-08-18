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

    pub fn clause_detail(&self, name: &str) -> Option<&ClauseDetail> {
        self.details.get(name)
    }

    fn is_computed_value(&self, term: &Term) -> bool {
        let mut pending = vec![term];
        while let Some(term) = pending.pop() {
            if let Term::App(name, arguments) = term {
                if self.functions.contains(name) {
                    return false;
                }
                pending.extend(arguments);
            }
        }
        true
    }

    fn compute(&self, term: &Term, fuel: usize) -> (Option<Term>, usize) {
        if !term.variables().is_empty() {
            return (None, 0);
        }
        let result = self.conditions.evaluate(
            term,
            &Config {
                fuel: fuel.min(soda_semantics::DEFAULT_FUEL),
                ..Config::default()
            },
        );
        let value = if result.status == Status::NormalForm
            && result.term.variables().is_empty()
            && self.is_computed_value(&result.term)
            && (!self.schema.is_typed() || self.schema.check_equation(term, &result.term).is_ok())
        {
            Some(result.term)
        } else {
            None
        };
        (value, result.steps)
    }

    fn check_condition(&self, term: &Term, fuel: usize) -> (Condition, usize) {
        if !term.variables().is_empty() {
            return (Condition::Unknown, 0);
        }

        let result = self.conditions.evaluate(
            term,
            &Config {
                fuel: fuel.min(soda_semantics::DEFAULT_FUEL),
                ..Config::default()
            },
        );
        let condition = match (result.status, result.term) {
            (Status::NormalForm, Term::Bool(true)) => Condition::True,
            (Status::NormalForm, Term::Bool(false)) => Condition::False,
            _ => Condition::Unknown,
        };

        (condition, result.steps)
    }

    /// Invalid typed queries return no answers and `complete = false`.
    /// Use [`Self::search_checked`] for the diagnostic.
    pub fn search(&self, query: &Term, config: &SearchConfig) -> SearchResult {
        self.search_checked(query, config)
            .unwrap_or_else(|_| SearchResult {
                answers: Vec::new(),
                complete: false,
                attempts: 0,
            })
    }

    pub fn search_checked(
        &self,
        query: &Term,
        config: &SearchConfig,
    ) -> Result<SearchResult, String> {
        if self.schema.is_typed() || soda_sorts::requires_sort_check(query) {
            self.schema
                .check_judgment(query)
                .map_err(|error| error.to_string())?;
        }

        Ok(self.search_unchecked(query, config))
    }

    fn search_unchecked(&self, query: &Term, config: &SearchConfig) -> SearchResult {
        let mut used = query.variables();

        for variables in &self.variables {
            used.extend(variables.iter().cloned());
        }

        let mut search = Search {
            relations: self,
            config,
            attempts: 0,
            complete: true,
            fresh: 0,
            used,
        };
        let results = search.goal(query, &Substitution::new(), 0);
        let answers = results
            .into_iter()
            .map(|(env, mut derivation)| {
                instantiate_derivation(&mut derivation, &env);

                Answer {
                    term: query.substitute(&env),
                    bindings: query
                        .variables()
                        .into_iter()
                        .map(|name| {
                            let value = Term::Var(name.clone()).substitute(&env);

                            (name, value)
                        })
                        .collect(),
                    derivation,
                }
            })
            .collect();

        SearchResult {
            answers,
            complete: search.complete,
            attempts: search.attempts,
        }
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

fn compose(env: &Substitution, next: &Substitution) -> Substitution {
    let mut result: BTreeMap<_, _> = env
        .iter()
        .map(|(name, value)| (name.clone(), value.substitute(next)))
        .collect();
    result.extend(next.clone());

    result
}

fn instantiate_derivation(proof: &mut Derivation, env: &Substitution) {
    proof.conclusion = proof.conclusion.substitute(env);
    for value in proof.substitution.values_mut() {
        *value = value.substitute(env);
    }

    for child in &mut proof.premises {
        instantiate_derivation(child, env);
    }
}

struct Search<'a> {
    relations: &'a Relations,
    config: &'a SearchConfig,
    attempts: usize,
    complete: bool,
    fresh: usize,
    used: BTreeSet<String>,
}

pub fn answer_matches(query: &Term, answer: &Answer) -> bool {
    match_term(query, &answer.term).is_some() && answer.term == answer.derivation.conclusion
}
