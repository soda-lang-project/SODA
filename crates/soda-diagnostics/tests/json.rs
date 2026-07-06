use soda_diagnostics::{Diagnostic, Report, SCHEMA_VERSION, Span, Suggestion};

#[test]
fn structured_output_round_trips_controls_unicode_and_missing_sources() {
    let mut report = Report::default();
    let source = report.sources.add("quoted\"\\file.soda", "α\tβ\n");
    let message = "quote\" slash\\ newline\n tab\t control\u{0000} emoji 🥤";
    let mut diagnostic = Diagnostic::error("E_TEST", message)
        .primary(source, Span::new(0, 2), message)
        .secondary(usize::MAX, Span::new(9, 10), "unknown source")
        .note(message)
        .help(message);
    diagnostic.suggestions.push(Suggestion {
        source,
        span: Span::new(0, 2),
        replacement: message.into(),
        message: message.into(),
    });
    report.diagnostics.push(diagnostic);
    let json = report.to_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(value["version"], SCHEMA_VERSION);
    let diagnostic = &value["diagnostics"][0];

    assert_eq!(diagnostic["message"], message);
    assert_eq!(diagnostic["notes"][0], message);
    assert_eq!(diagnostic["suggestions"][0]["replacement"], message);
    assert_eq!(diagnostic["labels"][0]["end"], 2);
    assert_eq!(diagnostic["labels"][0]["end_column"], 2);
    assert!(diagnostic["labels"][1]["source"].is_null());
    assert!(diagnostic["labels"][1].get("start").is_none());
    assert!(json.ends_with('\n'));
}
