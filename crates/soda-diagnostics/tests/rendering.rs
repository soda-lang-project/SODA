use soda_diagnostics::{
    Diagnostic, RenderOptions, Report, Severity, SourceFile, SourceMap, Span, Style, Suggestion,
    nearest, render,
};

#[test]
fn source_positions_handle_crlf_unicode_eof_and_invalid_byte_boundaries() {
    let source = SourceFile::new("source", "α\r\n漢字\n");

    assert_eq!(source.position(0), (1, 1));
    assert_eq!(source.position(4), (2, 1));
    assert_eq!(source.position(source.text.len()), (3, 1));
    assert_eq!(source.clamp(Span::new(1, 1)), Span::new(0, 2));
    assert_eq!(
        source.clamp(Span::new(usize::MAX, 0)).start,
        source.text.len()
    );
}

#[test]
fn human_diagnostic_snapshot_has_source_caret_help_and_no_ansi() {
    let mut sources = SourceMap::default();
    let source = sources.add("example.soda", "eval add(1, false);\n");
    let diagnostic = Diagnostic::error("E1001", "this expression has the wrong sort")
        .primary(source, Span::new(12, 17), "expected `Int`, found `Bool`")
        .help("pass an integer as the second argument");
    let output = render(
        &diagnostic,
        &sources,
        &RenderOptions {
            context_lines: 0,
            ..RenderOptions::default()
        },
    );

    assert_eq!(
        output,
        "error[E1001]: this expression has the wrong sort\n --> example.soda:1:13\n  |\n1 | eval add(1, false);\n  |             ^^^^^ expected `Int`, found `Bool`\n  |\n  = help: pass an integer as the second argument\n"
    );
    assert!(!output.contains('\x1b'));
}

#[test]
fn ascii_and_color_are_independent_and_controls_are_never_executed() {
    let mut sources = SourceMap::default();
    let source = sources.add("evil\x1b[2J.soda", "eval bad\x1b[2J;\n");
    let diagnostic = Diagnostic::error("E1", "bad\x1b[2J\u{202e}").primary(
        source,
        Span::new(5, 8),
        "this token",
    );
    let output = render(
        &diagnostic,
        &sources,
        &RenderOptions {
            style: Style::Ascii,
            color: true,
            ..RenderOptions::default()
        },
    );

    assert!(
        output.contains("\x1b[") && output.contains("error") && output.contains("[E1]"),
        "{output}"
    );
    assert!(!output.contains("\x1b[2J"));
    assert!(!output.contains('\u{202e}'));
    assert!(output.contains("evil\\u{1b}[2J.soda"));
    assert!(!output.contains('│'));
}

#[test]
fn display_columns_account_for_tabs_wide_characters_and_combining_marks() {
    let mut sources = SourceMap::default();
    let text = "\t漢 e\u{301} false";
    let source = sources.add("unicode", text);
    let start = text.find("false").unwrap();
    let output = render(
        &Diagnostic::error("E1", "unicode").primary(source, Span::new(start, start + 5), "here"),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("|          ^^^^^ here"), "{output}");
    assert!(output.contains("    漢 e\u{301} false"));
}

#[test]
fn long_source_lines_are_windowed_around_the_label() {
    let mut sources = SourceMap::default();
    let text = format!("{}bad{}", "x".repeat(500), "y".repeat(500));
    let source = sources.add("long", text);
    let output = render(
        &Diagnostic::error("E1", "long").primary(source, Span::new(500, 503), "bad"),
        &sources,
        &RenderOptions {
            width: 60,
            ..RenderOptions::default()
        },
    );

    assert!(output.contains("bad"));
    assert!(output.contains("..."));
    assert!(output.len() < 350, "{output}");
}

#[test]
fn distant_labels_on_one_line_have_separate_truthful_source_windows() {
    let mut sources = SourceMap::default();
    let text = format!("first {} last", "x".repeat(500));
    let end = text.len();
    let source = sources.add("distant", text);
    let diagnostic = Diagnostic::error("E1", "two locations")
        .primary(source, Span::new(end - 4, end), "last token")
        .secondary(source, Span::new(0, 5), "first token");
    let output = render(
        &diagnostic,
        &sources,
        &RenderOptions {
            width: 60,
            ..RenderOptions::default()
        },
    );

    assert!(output.contains("first "));
    assert!(output.contains(" last"));
    assert!(output.contains("^^^^ last token"), "{output}");
    assert!(output.contains("----- first token"));
    assert!(output.len() < 500, "{output}");
}

