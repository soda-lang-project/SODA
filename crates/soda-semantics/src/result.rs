use soda_ast::Term;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Fuel,
    Depth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    NormalForm,
    Stuck(String),
    Error(Box<crate::RuntimeError>),
    Limit(Limit),
}

/// A contraction at `path`. `before` and `after` are the subterms at that path.
/// Guard contractions consume fuel but are excluded from the observable trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    pub rule: String,
    pub path: Vec<usize>,
    pub before: Term,
    pub after: Term,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub term: Term,
    pub status: Status,
    pub steps: usize,
    pub trace: Vec<TraceStep>,
    /// Last observable user rule, retained even when full trace recording is off.
    pub last_rule: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub rule: String,
    pub message: String,
}
impl ValidationError {
    pub(crate) fn new(rule: &str, message: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rule {} {}", self.rule, self.message)
    }
}

impl std::error::Error for ValidationError {}
