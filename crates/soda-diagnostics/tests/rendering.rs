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

