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

