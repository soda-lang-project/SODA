use soda_ast::Term;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationErrorKind {
    Declaration,
    Step,
    Substitution,
    Position,
    Comparison,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationError {
    pub kind: VerificationErrorKind,
    pub declaration: String,
    /// Zero-based certificate step, when applicable.
    pub step: Option<usize>,
    pub message: String,
    pub expected: Option<Box<Term>>,
    pub found: Option<Box<Term>>,
}

impl VerificationError {
    pub(crate) fn declaration(name: &str, message: impl Into<String>) -> Self {
        Self {
            kind: VerificationErrorKind::Declaration,
            declaration: name.into(),
            step: None,
            message: message.into(),
            expected: None,
            found: None,
        }
    }

    pub(crate) fn step(name: &str, step: usize, message: impl Into<String>) -> Self {
        Self {
            kind: VerificationErrorKind::Step,
            declaration: name.into(),
            step: Some(step),
            message: message.into(),
            expected: None,
            found: None,
        }
    }

}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.declaration)?;
        if let Some(step) = self.step {
            write!(f, " step {}", step + 1)?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for VerificationError {}
