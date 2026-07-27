use soda_ast::Term;
use soda_parser::{parse_module, parse_term};

#[test]
fn parses_complete_semantics_and_certificate() {
    let source = r#"
        # arithmetic
        module Arithmetic;
        rule addition: add(?x, ?y) => sum(?x, ?y) when natural(?x);
        axiom identity: add(?x, 0) = ?x;
        eval add(42, -1);
        proof witness: wrap(add(2, 0)) = wrap(2) {
            identity at [0] with {x = 2};
            identity reverse at [0] with {?x = 2}; // explicit direction
            identity at [0] with {x = 2};
        };
    "#;
    let module = parse_module(source).unwrap();

    assert_eq!(module.name, "Arithmetic");
    assert_eq!(module.rules.len(), 1);
    assert_eq!(
        module.rules[0].condition,
        Some(Term::app("natural", vec![Term::Var("x".into())]))
    );
    assert_eq!(module.equations.len(), 1);
    assert_eq!(module.evaluations.len(), 1);
    let proof = &module.proofs[0];

    assert_eq!(proof.steps.len(), 3);
    assert!(proof.steps[1].reverse);
    assert_eq!(proof.steps[0].path, vec![0]);
    assert_eq!(proof.steps[0].substitution["x"], Term::Int(2));
}

#[test]
fn literals_and_zero_arity_constructors() {
    for (source, expected) in [
        ("-9223372036854775808", Term::Int(i64::MIN)),
        ("9223372036854775807", Term::Int(i64::MAX)),
        ("+123", Term::Int(123)),
        ("true", Term::Bool(true)),
        ("false", Term::Bool(false)),
        ("nil()", Term::app("nil", vec![])),
        ("nil", Term::app("nil", vec![])),
        ("?value_2", Term::Var("value_2".into())),
    ] {
        assert_eq!(parse_term(source).unwrap(), expected, "{source}");
    }
}

#[test]
fn terms_round_trip_through_display() {
    let term = parse_term("state(pair(?x, -3), true, nil())").unwrap();

    assert_eq!(parse_term(&term.to_string()).unwrap(), term);
}

#[test]
fn rejects_bad_terms_without_panics() {
    for source in [
        "",
        "?",
        "f(",
        "f(a,)",
        "f(a b)",
        "f(a))",
        "1 2",
        "true()",
        "-",
        "+",
        "🦀",
        "9223372036854775808",
        "-9223372036854775809",
        "a => b",
    ] {
        assert!(parse_term(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn rejects_malformed_modules() {
    for source in [
        "",
        "module A",
        "module A; nonsense hi;",
        "module A; rule r: a = b;",
        "module A; axiom r: a => b;",
        "module A; eval f(a,);",
        "module A; proof p: a = b { ax at [-1]; }",
        "module A; proof p: a = b { ax at [+1]; }",
        "module A; proof p: a = b { ax at [999999999999999999999999999999]; }",
        "module A; proof p: a = b { ax at [0,]; }",
        "module A; proof p: a = b { ax with {x = 1, x = 2}; }",
        "module A; proof p: a = b { ax;",
        "module A; proof p: a = b { ax }",
    ] {
        assert!(parse_module(source).is_err(), "accepted {source:?}");
    }
}

#[test]
fn rejects_duplicate_named_declarations() {
    let error = parse_module("module A; rule r: a => b; axiom r: a = b;").unwrap_err();

    assert_eq!(error.message, "duplicate declaration 'r'");
    assert_eq!(error.column, 33);
}

#[test]
fn errors_include_exact_source_positions() {
    let source = "module A;\n  eval f(1, );";
    let error = parse_module(source).unwrap_err();

    assert_eq!((error.line, error.column), (2, 13));
    assert_eq!(&source[error.offset..], ");");
    assert!(error.to_string().starts_with("2:13:"));
    let unicode_error = parse_term("# αβ\n %").unwrap_err();

    assert_eq!((unicode_error.line, unicode_error.column), (2, 2));
}

#[test]
fn proof_defaults_and_empty_certificates() {
    let module =
        parse_module("module M; proof p: a = a {} proof q: a = b { ax reverse; }").unwrap();

    assert!(module.proofs[0].steps.is_empty());
    let step = &module.proofs[1].steps[0];

    assert!(step.reverse);
    assert!(step.path.is_empty());
    assert!(step.substitution.is_empty());
}

#[test]
fn excessive_nesting_is_a_diagnostic() {
    let source = format!("{}x{}", "f(".repeat(300), ")".repeat(300));

    assert!(parse_term(&source).unwrap_err().message.contains("nesting"));
}

