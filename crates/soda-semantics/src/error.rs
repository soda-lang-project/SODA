#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    InvalidOperands,
    Arity,
    DivisionByZero,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
    pub guards: Vec<String>,
}
impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: RuntimeErrorKind::InvalidOperands,
            message: message.into(),
            guards: Vec::new(),
        }
    }

    pub(crate) fn with_kind(mut self, kind: RuntimeErrorKind) -> Self {
        self.kind = kind;
        self
    }

    pub(crate) fn in_guard(mut self, name: &str) -> Self {
        self.guards.push(name.into());
        self
    }
}

impl From<String> for RuntimeError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for RuntimeError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for name in self.guards.iter().rev() {
            write!(formatter, "guard of {name}: ")?;
        }

        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeError {}
