mod json;
mod spelling;
pub use json::SCHEMA_VERSION;
pub use spelling::nearest;
mod render;
mod source;

pub use render::{DEFAULT_RENDER_WIDTH, MAX_RENDER_WIDTH, MIN_RENDER_WIDTH, RenderOptions, Style, safe as escape_terminal};
pub use source::{SourceFile, SourceId, SourceMap, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub source: SourceId,
    pub span: Span,
    pub message: String,
    pub primary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub source: SourceId,
    pub span: Span,
    pub replacement: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub title: String,
    pub labels: Vec<Label>,
    pub notes: Vec<String>,
    pub help: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            title: title.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    pub fn primary(mut self, source: SourceId, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            source,
            span,
            message: message.into(),
            primary: true,
        });
        self
    }

    pub fn secondary(mut self, source: SourceId, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            source,
            span,
            message: message.into(),
            primary: false,
        });
        self
    }

    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(message.into());
        self
    }

    pub fn help(mut self, message: impl Into<String>) -> Self {
        self.help.push(message.into());
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {

    pub fn to_json(&self) -> String {
        json::report(self)
    }
}
