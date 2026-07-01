#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefinitionError<Kind> {
    pub kind: Kind,
    pub declaration: Option<String>,
    pub message: String,
}

impl<Kind: Default> DefinitionError<Kind> {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::default(),
            declaration: None,
            message: message.into(),
        }
    }
}

impl<Kind> DefinitionError<Kind> {
    pub fn with_kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    pub fn in_declaration(mut self, name: impl Into<String>) -> Self {
        if self.declaration.is_none() {
            self.declaration = Some(name.into());
        }

        self
    }
}

impl<Kind: Default> From<String> for DefinitionError<Kind> {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl<Kind: Default> From<&str> for DefinitionError<Kind> {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

impl<Kind> std::fmt::Display for DefinitionError<Kind> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.declaration {
            write!(formatter, "{name}: ")?;
        }

        formatter.write_str(&self.message)
    }
}

impl<Kind: std::fmt::Debug> std::error::Error for DefinitionError<Kind> {}
