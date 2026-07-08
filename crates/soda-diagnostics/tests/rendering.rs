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

#[test]
fn multiline_labels_collapse_distant_lines_and_have_an_end_message() {
    let mut sources = SourceMap::default();
    let text = "x\n".repeat(1000);
    let source = sources.add("many", &text);
    let output = render(
        &Diagnostic::error("E1", "range").primary(
            source,
            Span::new(0, text.len() - 1),
            "ends here",
        ),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("..."));
    assert!(output.contains("ends here"));
    assert!(output.len() < 1000);
}

#[test]
fn related_labels_support_multiple_files_and_secondary_style() {
    let mut sources = SourceMap::default();
    let use_site = sources.add("argument", "false");
    let declaration = sources.add("module", "function f(Int): Int;");
    let output = render(
        &Diagnostic::error("E1", "mismatch")
            .primary(use_site, Span::new(0, 5), "wrong sort")
            .secondary(declaration, Span::new(11, 14), "expected here"),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("argument:1:1"));
    assert!(output.contains("module:1:12"));
    assert!(output.contains("--- expected here"));
}

#[test]
fn json_is_deterministic_escaped_and_contains_machine_locations_and_fixes() {
    let mut report = Report::default();
    let source = report.sources.add("a\"b", "λ\tbad");
    let mut diagnostic =
        Diagnostic::error("E1", "\"message\"\n\x00").primary(source, Span::new(3, 6), "label");
    diagnostic.severity = Severity::Warning;
    diagnostic.suggestions.push(Suggestion {
        source,
        span: Span::new(3, 6),
        replacement: "good".into(),
        message: "replace it".into(),
    });
    report.diagnostics.push(diagnostic);
    let json = report.to_json();

    assert!(json.starts_with("{\"version\":1,\"diagnostics\":["));
    assert!(json.contains("\"severity\":\"warning\""));
    assert!(json.contains("\"start\":3,\"end\":6,\"line\":1,\"column\":3"));
    assert!(json.contains("\\\"message\\\"\\n\\u0000"));
    assert!(json.contains("\"replacement\":\"good\""));
    assert!(!json.contains('\x1b'));
    assert_eq!(json, report.to_json());
}

#[test]
fn invalid_spans_and_unknown_sources_are_safe() {
    let mut sources = SourceMap::default();
    let source = sources.add("empty", "");
    let output = render(
        &Diagnostic::error("E1", "end")
            .primary(source, Span::new(usize::MAX, 0), "EOF")
            .secondary(999, Span::default(), "missing"),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("empty:1:1"));
    assert!(output.contains("^ EOF"));
}

#[test]
fn suggestions_are_conservative_and_deterministic() {
    assert_eq!(nearest("addd", ["mul", "add", "and"]), Some("add"));
    assert_eq!(nearest("completely_unrelated", ["mul", "add"]), None);
    assert_eq!(nearest("a", ["b", "c"]), None);
}

#[test]
fn unicode_style_uses_connected_gutters_and_primary_markers() {
    let mut sources = SourceMap::default();
    let source = sources.add("unicode.soda", "eval false;\n");
    let output = render(
        &Diagnostic::error("E1", "wrong sort").primary(source, Span::new(5, 10), "expected Int"),
        &sources,
        &RenderOptions {
            style: Style::Unicode,
            ..RenderOptions::default()
        },
    );

    assert!(output.contains("╭▸ unicode.soda:1:6"), "{output}");
    assert!(output.contains("━━━━━ expected Int"), "{output}");
    assert!(!output.contains('\x1b'));
}

#[test]
fn patch_previews_support_multiline_replacements_and_keep_original_json_ranges() {
    let mut report = Report::default();
    let source = report.sources.add("patch.soda", "eval bad;\neval 1;\n");
    let mut diagnostic =
        Diagnostic::error("E1", "replace this term").primary(source, Span::new(5, 8), "bad term");
    diagnostic.suggestions.push(Suggestion {
        source,
        span: Span::new(5, 8),
        replacement: "good;\neval better".into(),
        message: "use these inputs".into(),
    });
    report.diagnostics.push(diagnostic);
    let output = report.render(&RenderOptions::default());

    assert!(output.contains("help: use these inputs"), "{output}");
    assert!(output.contains("eval good;"), "{output}");
    assert!(output.contains("eval better;"), "{output}");
    assert!(report.to_json().contains("\"start\":5,\"end\":8"));
}

#[test]
fn eof_after_a_newline_and_labels_after_escaped_controls_remain_renderable() {
    let mut sources = SourceMap::default();
    let source = sources.add("eof.soda", "eval 1;\n");
    let output = render(
        &Diagnostic::error("E1", "unexpected end").primary(
            source,
            Span::new(8, 8),
            "expected a term",
        ),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("eof.soda:2:1"), "{output}");
    let text = "\x1b[2J false";
    let source = sources.add("safe.soda", text);
    let start = text.find("false").unwrap();
    let output = render(
        &Diagnostic::error("E1", "bad input").primary(
            source,
            Span::new(start, start + 5),
            "this Boolean",
        ),
        &sources,
        &RenderOptions::default(),
    );

    assert!(output.contains("\\u{1b}[2J false"), "{output}");
    assert!(output.contains("^^^^^ this Boolean"), "{output}");
    assert!(!output.contains('\x1b'));
}
