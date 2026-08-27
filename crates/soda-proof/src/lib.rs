mod error;
pub use error::{InductionError, InductionErrorKind};
mod correspondence;
pub use correspondence::{Correspondence, check_correspondence};

use soda_ast::{Equation, InductionProof, Module, Proof, SignatureKind, Substitution, Term};
use soda_sorts::TypeChecker;
use soda_verifier::{verify_module, verify_proof};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedTheorem {
    pub name: String,
    pub cases: usize,
    pub steps: usize,
}

