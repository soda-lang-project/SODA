use soda_ast::Term;
use soda_parser::{parse_module, parse_module_located, parse_term_located_in};

#[test]
fn exact_subterm_and_declaration_ranges_survive_comments_crlf_and_unicode() {
    let text = "module Μ;\r\n# true is a comment\r\nfunction f(Int): Int;\r\nrule f: f(?n) => add(?n, true);\r\n";
    let parsed = parse_module_located(text).unwrap();

    assert_eq!(parsed.module, parse_module(text).unwrap());
    let rule = parsed
        .locations
        .declarations
        .iter()
        .find(|site| site.kind == soda_parser::DeclarationKind::Rule)
        .unwrap();

    assert_eq!(&text[rule.name_span.start..rule.name_span.end], "f");
    let span = parsed
        .locations
        .term(&Term::Bool(true), Some(rule.span))
        .unwrap();

    assert_eq!(&text[span.start..span.end], "true");
    assert_eq!(span.start, text.rfind("true").unwrap());
}

#[test]
fn notation_ranges_point_to_the_original_math_and_not_its_lowered_text() {
    let module = parse_module(
        "module N; judgment step(Expr,Expr); notation step \"$0 → $1\" ascii \"$0 -> $1\";",
    )
    .unwrap();
    let text = "a → f(false)";
    let parsed = parse_term_located_in(&module, text).unwrap();
    let whole = parsed.locations.term(&parsed.term, None).unwrap();

    assert_eq!(&text[whole.start..whole.end], text);
    let argument = parsed.locations.term(&Term::Bool(false), None).unwrap();

    assert_eq!(&text[argument.start..argument.end], "false");
}

#[test]
fn induction_cases_and_proof_steps_have_their_own_ranges() {
    let text = "module P; induction p on ?n: Nat: f(?n) = ?n { case zero { base; } case succ(k) { ih_k with {k=?k}; } }";
    let parsed = parse_module_located(text).unwrap();
    let site = parsed
        .locations
        .declarations
        .iter()
        .find(|site| {
            site.kind == soda_parser::DeclarationKind::Case && site.name.as_deref() == Some("succ")
        })
        .unwrap();

    assert!(text[site.span.start..site.span.end].starts_with("case succ(k)"));
    assert_eq!(
        &text[parsed.locations.steps[1].start..parsed.locations.steps[1].end],
        "ih_k with {k=?k};"
    );
}

#[test]
fn missing_semicolon_suggests_a_real_parseable_edit() {
    let text = "module P; rule id: id(?x) => ?x\neval id(1);";
    let error = parse_module(text).unwrap_err();
    let diagnostic = error.diagnostic(0, text);
    let suggestion = diagnostic.suggestions.first().unwrap();
    let fixed = format!(
        "{}{}{}",
        &text[..suggestion.span.start],
        suggestion.replacement,
        &text[suggestion.span.end..]
    );

    assert!(parse_module(&fixed).is_ok(), "{fixed}");
    assert!(fixed.contains("?x;\neval"));
}

#[test]
fn grouped_constructors_retain_their_own_declaration_locations() {
    let source = "module M; datatype Value { constructor value(Int); constructor poison(); }";
    let parsed = parse_module_located(source).unwrap();
    for name in ["value", "poison"] {
        let site = parsed.locations.declaration(name).unwrap();
        assert_eq!(site.kind, soda_parser::DeclarationKind::Constructor);
        assert_eq!(&source[site.name_span.start..site.name_span.end], name);
        assert!(source[site.span.start..site.span.end].starts_with("constructor "));
    }
    let site = parsed.locations.declaration("Value").unwrap();
    assert!(source[site.span.start..site.span.end].ends_with('}'));
}

#[test]
fn function_cases_have_rule_locations_and_compact_definitions_have_function_locations() {
    let source = "module M; function f(Int): Int { case wrong: (?x) => false; } function g(?n: Int): Int => ?n;";
    let parsed = parse_module_located(source).unwrap();
    let case = parsed.locations.declaration("wrong").unwrap();
    assert_eq!(case.kind, soda_parser::DeclarationKind::Rule);
    assert_eq!(&source[case.name_span.start..case.name_span.end], "wrong");
    assert_eq!(
        &source[case.span.start..case.span.end],
        "case wrong: (?x) => false;"
    );
    let term = parsed
        .locations
        .term(&Term::Bool(false), Some(case.span))
        .unwrap();
    assert_eq!(&source[term.start..term.end], "false");
    let compact = parsed.locations.declaration("g").unwrap();
    assert_eq!(compact.kind, soda_parser::DeclarationKind::Function);
}
