use soda_diagnostics::SourceMap;
use soda_parser::{ParseErrorKind, parse_module, parse_term};

#[test]
fn diagnostic_codes_and_fixes_do_not_depend_on_message_wording() {
    let cases = [
        ("module M; eval 9223372036854775808;", "E0002"),
        ("module M; notation j \"unfinished", "E0004"),
        ("module M; rule same: a => b; rule same: b => c;", "E0005"),
    ];

    for (source, code) in cases {
        let mut error = parse_module(source).unwrap_err();
        error.message = "a replacement explanation".into();
        let mut sources = SourceMap::default();
        let id = sources.add("input.soda", source);

        assert_eq!(error.diagnostic(id, source).code, code);
    }

    let source = "module M; eval 42";
    let mut error = parse_module(source).unwrap_err();

    assert_eq!(error.kind, ParseErrorKind::ExpectedToken(";".into()));
    error.message = "a declaration needs a terminator".into();
    let mut sources = SourceMap::default();
    let id = sources.add("input.soda", source);

    assert_eq!(error.diagnostic(id, source).suggestions[0].replacement, ";");
}

