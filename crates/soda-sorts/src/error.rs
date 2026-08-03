use soda_ast::Term;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortErrorKind {
    InvalidDefinition,
    Mismatch,
    DuplicateDeclaration,
    UnboundName,
    Arity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortError {
    pub kind: SortErrorKind,
    pub message: String,
    pub declaration: Option<Box<str>>,
    pub term: Option<Box<Term>>,
    pub expected: Option<Box<str>>,
    pub found: Option<Box<str>>,
    pub symbol: Option<String>,
}

impl SortError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            kind: SortErrorKind::InvalidDefinition,
            declaration: None,
            term: None,
            expected: None,
            found: None,
            symbol: None,
        }
    }

    pub(crate) fn with_kind(mut self, kind: SortErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn at(mut self, term: &Term) -> Self {
        if self.term.is_none() {
            self.term = Some(Box::new(term.clone()));
        }

        self
    }

    pub fn in_declaration(mut self, name: &str) -> Self {
        if self.declaration.is_none() {
            self.declaration = Some(name.into());
        }

        self
    }

    pub fn symbol(mut self, name: &str) -> Self {
        self.symbol = Some(name.into());
        self
    }

    pub fn mismatch(expected: String, found: String) -> Self {
        let mut error = Self::new(format!("sort mismatch: {found} versus {expected}"));
        error.kind = SortErrorKind::Mismatch;
        error.expected = Some(expected.into_boxed_str());
        error.found = Some(found.into_boxed_str());

        error
    }
}

impl fmt::Display for SortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SortError {}
