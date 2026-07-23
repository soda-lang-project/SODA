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

