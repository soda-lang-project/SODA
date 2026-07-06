use crate::{Diagnostic, Report, SourceId, SourceMap, Span};
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct ReportView<'a> {
    version: u32,
    diagnostics: Vec<DiagnosticView<'a>>,
}

#[derive(Serialize)]
struct DiagnosticView<'a> {
    severity: &'static str,
    code: &'a str,
    message: &'a str,
    labels: Vec<LabelView<'a>>,
    notes: &'a [String],
    help: &'a [String],
    suggestions: Vec<SuggestionView<'a>>,
}

#[derive(Serialize)]
struct LabelView<'a> {
    #[serde(flatten)]
    location: Location<'a>,
    message: &'a str,
    primary: bool,
}

#[derive(Serialize)]
struct SuggestionView<'a> {
    #[serde(flatten)]
    location: Location<'a>,
    replacement: &'a str,
    message: &'a str,
    applicability: &'static str,
}

#[derive(Serialize)]
struct Location<'a> {
    source: Option<&'a str>,
    #[serde(flatten)]
    position: Option<Position>,
}

#[derive(Serialize)]
struct Position {
    start: usize,
    end: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
}

impl<'a> Location<'a> {
    fn new(sources: &'a SourceMap, id: SourceId, span: Span) -> Self {
        let Some(source) = sources.get(id) else {
            return Self {
                source: None,
                position: None,
            };
        };

        let span = source.clamp(span);
        let (line, column) = source.position(span.start);
        let (end_line, end_column) = source.position(span.end);

        Self {
            source: Some(&source.name),
            position: Some(Position {
                start: span.start,
                end: span.end,
                line,
                column,
                end_line,
                end_column,
            }),
        }
    }
}

impl<'a> DiagnosticView<'a> {
    fn new(diagnostic: &'a Diagnostic, sources: &'a SourceMap) -> Self {
        Self {
            severity: diagnostic.severity.as_str(),
            code: &diagnostic.code,
            message: &diagnostic.title,
            labels: diagnostic
                .labels
                .iter()
                .map(|label| LabelView {
                    location: Location::new(sources, label.source, label.span),
                    message: &label.message,
                    primary: label.primary,
                })
                .collect(),
            notes: &diagnostic.notes,
            help: &diagnostic.help,
            suggestions: diagnostic
                .suggestions
                .iter()
                .map(|suggestion| SuggestionView {
                    location: Location::new(sources, suggestion.source, suggestion.span),
                    replacement: &suggestion.replacement,
                    message: &suggestion.message,
                    applicability: "suggested",
                })
                .collect(),
        }
    }
}

pub(crate) fn report(report: &Report) -> String {
    let view = ReportView {
        version: SCHEMA_VERSION,
        diagnostics: report
            .diagnostics
            .iter()
            .map(|diagnostic| DiagnosticView::new(diagnostic, &report.sources))
            .collect(),
    };
    let mut json = serde_json::to_string(&view)
        .expect("the diagnostic wire schema contains only JSON-safe values");
    json.push('\n');

    json
}
