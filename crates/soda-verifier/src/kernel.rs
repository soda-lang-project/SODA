use std::collections::{BTreeMap, BTreeSet};

use soda_ast::{Equation, Module, Proof, Term};

use crate::validation::{validate_equations, validate_proof_header, validate_term};
use crate::{VerificationError, VerificationErrorKind, validate_module};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedProof {
    pub name: String,
    pub steps_checked: usize,
    pub conclusion: (Term, Term),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub axioms_checked: usize,
    pub rules_checked: usize,
    pub proofs: Vec<VerifiedProof>,
}

