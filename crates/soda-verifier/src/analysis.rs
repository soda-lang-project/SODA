use std::collections::BTreeMap;

use crate::validation::validate_term;
use soda_ast::{Rule, Substitution, Term};

/// A result for syntactic rewriting; failure to prove termination means unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminationAnalysis {
    pub proven: bool,
    pub unproven_rules: Vec<String>,
}

