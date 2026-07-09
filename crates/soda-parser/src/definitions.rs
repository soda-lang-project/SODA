use crate::{Kind, ParseError, ParseErrorKind, Parser};
use soda_ast::{Clause, ClauseDetail, ComputedBinding, Equation, Module, Proof, Rule, Term};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DefinitionKind {
    Rule,
    Axiom,
    Proof,
    Infer,
}

impl Parser<'_> {

}
