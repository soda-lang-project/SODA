use soda_ast::{Module, Rule, Substitution, Term};
use std::collections::{BTreeMap, BTreeSet};

use crate::validation::validate_term;
use crate::{VerificationError, validate_module};

/// A syntactic peak with variables freshened independently of other pairs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriticalPair {
    pub outer_rule: String,
    pub inner_rule: String,
    /// The inner contraction position; the outer contraction is at the root.
    pub position: Vec<usize>,
    pub source: Term,
    pub outer_reduct: Term,
    pub inner_reduct: Term,
}

/// Syntactic overlap information, not a confluence or joinability verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriticalPairAnalysis {
    pub pairs: Vec<CriticalPair>,
    pub excluded_conditional_rules: Vec<String>,
    pub limitations: Vec<String>,
}

