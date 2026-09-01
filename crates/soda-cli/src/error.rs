#[derive(Debug)]
pub(crate) enum CommandError {
    Validation(String),
    Engine(soda_semantics::ValidationError),
    Domain(soda_domains::DomainError),
    Induction(soda_proof::InductionError),
}
impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self::Validation(message)
    }
}

impl From<&str> for CommandError {
    fn from(message: &str) -> Self {
        Self::Validation(message.into())
    }
}

impl From<soda_domains::DomainError> for CommandError {
    fn from(error: soda_domains::DomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<soda_proof::InductionError> for CommandError {
    fn from(error: soda_proof::InductionError) -> Self {
        Self::Induction(error)
    }
}

impl From<soda_semantics::ValidationError> for CommandError {
    fn from(error: soda_semantics::ValidationError) -> Self {
        Self::Engine(error)
    }
}

impl CommandError {
    pub fn message(&self) -> &str {
        match self {
            Self::Validation(message) => message,
            Self::Engine(error) => &error.message,
            Self::Domain(error) => &error.message,
            Self::Induction(error) => &error.message,
        }
    }

    pub fn declaration(&self) -> Option<&str> {
        match self {
            Self::Domain(error) => error.declaration.as_deref(),
            Self::Induction(error) => error.declaration.as_deref(),
            Self::Validation(_) => None,
            Self::Engine(error) => Some(&error.rule),
        }
    }
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) => formatter.write_str(message),
            Self::Engine(error) => error.fmt(formatter),
            Self::Domain(error) => error.fmt(formatter),
            Self::Induction(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(_) => None,
            Self::Engine(error) => Some(error),
            Self::Domain(error) => Some(error),
            Self::Induction(error) => Some(error),
        }
    }
}
