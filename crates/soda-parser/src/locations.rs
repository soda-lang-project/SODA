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
    pub const fn is_top_level(self) -> bool {
        !matches!(self, Self::Module | Self::Case)
    }

    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Some(match keyword {
            "module" => Self::Module,
            "record" => Self::Record,
            "monad" => Self::Monad,
            "import" => Self::Import,
            "domain" => Self::Domain,
            "map" => Self::Map,
            "fixed" => Self::Fixed,
            "induction" => Self::Induction,
            "sort" => Self::Sort,
            "datatype" => Self::Datatype,
            "constructor" => Self::Constructor,
            "function" => Self::Function,
            "judgment" => Self::Judgment,
            "notation" => Self::Notation,
            "eval" => Self::Eval,
            "query" => Self::Query,
            "infer" => Self::Infer,
            "rule" => Self::Rule,
            "axiom" => Self::Axiom,
            "proof" => Self::Proof,
            "case" => Self::Case,
            _ => return None,
        })
    }

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

    pub const fn is_input(self) -> bool {
        matches!(self, Self::Eval | Self::Query)
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
    pub fn declaration(&self, name: &str) -> Option<&DeclarationLocation> {
        self.declarations
            .iter()
            .find(|declaration| declaration.name.as_deref() == Some(name))
    }

    pub fn term(&self, term: &Term, within: Option<Span>) -> Option<Span> {
        self.terms
            .iter()
            .filter(|site| within.is_none_or(|span| span.contains(site.span)) && site.term == *term)
            .min_by_key(|site| site.span.end - site.span.start)
            .map(|site| site.span)
    }
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
    pub(super) fn record_term(&mut self, start: usize, term: &Term) {
        if !self.track_locations {
            return;
        }

        let span = Span::new(start, self.previous_end());

        if !self
            .locations
            .terms
            .iter()
            .rev()
            .take(RECENT_TERM_LOCATIONS)
            .any(|site| site.span == span && site.term == *term)
        {
            self.locations.terms.push(TermLocation {
                term: term.clone(),
                span,
            });
        }
    }

    pub(super) fn record_declaration(&mut self, start: usize) {
        if !self.track_locations {
            return;
        }

        let token = &self.tokens[start];
        let Kind::Ident(kind) = &token.kind else {
            return;
        };

        let Some(kind) = DeclarationKind::from_keyword(kind) else {
            return;
        };

        let next = &self.tokens[start + 1];
        let name = if kind.is_input() {
            None
        } else {
            match &next.kind {
                Kind::Ident(name) => Some(name.clone()),
                _ => None,
            }
        };

        self.locations.declarations.push(DeclarationLocation {
            kind,
            name,
            span: Span::new(token.offset, self.previous_end()),
            name_span: Span::new(next.offset, next.end),
        });
    }
}
