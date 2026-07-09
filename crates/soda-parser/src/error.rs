use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    Syntax,
    ExpectedSyntax,
    ExpectedToken(String),
    IntegerRange,
    NestingLimit,
    UnterminatedString,
    DuplicateDeclaration(String),
    UnknownDeclaration(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl ParseError {
    pub(crate) fn with_kind(mut self: Box<Self>, kind: ParseErrorKind) -> Box<Self> {
        self.kind = kind;
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

pub(crate) fn error(source: &str, offset: usize, message: impl Into<String>) -> Box<ParseError> {
    let prefix = &source[..offset];

    Box::new(ParseError {
        kind: ParseErrorKind::Syntax,
        offset,
        line: prefix.bytes().filter(|b| *b == b'\n').count() + 1,
        column: prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        message: message.into(),
    })
}
