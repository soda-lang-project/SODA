use soda_ast::{Module, Term};
use soda_diagnostics::Span;

const RECENT_TERM_LOCATIONS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationKind {
    Module,
    Record,
    Monad,
    Import,
    Domain,
    Map,
    Fixed,
    Induction,
    Sort,
    Datatype,
    Constructor,
    Function,
    Judgment,
    Notation,
    Eval,
    Query,
    Infer,
    Rule,
    Axiom,
    Proof,
    Case,
}

impl std::fmt::Display for DeclarationKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl DeclarationKind {
    pub const TOP_LEVEL: &[Self] = &[
        Self::Record,
        Self::Monad,
        Self::Import,
        Self::Domain,
        Self::Map,
        Self::Fixed,
        Self::Induction,
        Self::Sort,
        Self::Datatype,
        Self::Constructor,
        Self::Function,
        Self::Judgment,
        Self::Notation,
        Self::Eval,
        Self::Query,
        Self::Infer,
        Self::Rule,
        Self::Axiom,
        Self::Proof,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Record => "record",
            Self::Monad => "monad",
            Self::Import => "import",
            Self::Domain => "domain",
            Self::Map => "map",
            Self::Fixed => "fixed",
            Self::Induction => "induction",
            Self::Sort => "sort",
            Self::Datatype => "datatype",
            Self::Constructor => "constructor",
            Self::Function => "function",
            Self::Judgment => "judgment",
            Self::Notation => "notation",
            Self::Eval => "eval",
            Self::Query => "query",
            Self::Infer => "infer",
            Self::Rule => "rule",
            Self::Axiom => "axiom",
            Self::Proof => "proof",
            Self::Case => "case",
        }
    }

}

#[derive(Clone, Debug)]
pub struct TermLocation {
    pub term: Term,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct DeclarationLocation {
    pub kind: DeclarationKind,
    pub name: Option<String>,
    pub span: Span,
    pub name_span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct Locations {
    pub terms: Vec<TermLocation>,
    pub declarations: Vec<DeclarationLocation>,
    pub steps: Vec<Span>,
}

impl Locations {

}

#[derive(Clone, Debug)]
pub struct ParsedModule {
    pub module: Module,
    pub locations: Locations,
}

#[derive(Clone, Debug)]
pub struct ParsedTerm {
    pub term: Term,
    pub locations: Locations,
}

use crate::{Kind, Parser};

impl Parser<'_> {

}
