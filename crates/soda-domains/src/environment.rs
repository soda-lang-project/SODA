use super::{Domains, FiniteDomain, Shape};
use crate::DomainError;
use soda_ast::Term;
use std::collections::BTreeMap;

/// Bindings retain their domain identity and distinguish names from metavariables.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    bindings: BTreeMap<String, (FiniteDomain, Term)>,
}

impl Environment {

}
