pub mod support;
use soda_ast::Term;
use soda_parser::{parse_module, parse_term};
use soda_relations::{Derivation, Relations, SearchConfig};

const EXAMPLE: &str = include_str!("../../../examples/computed-bindings.soda");

fn model(source: &str) -> Relations {
    Relations::from_module(&parse_module(source).unwrap()).unwrap()
}

#[test]
fn executable_example_computes_arithmetic_and_feeds_later_premises() {
    let parsed = parse_module(EXAMPLE).unwrap();
    let relations = Relations::from_module(&parsed).unwrap();
    for (query, expected) in parsed.queries.iter().zip([
        "evaluates(plus(lit(2), plus(lit(3), lit(4))), 9)",
        "pipeline(2, 16)",
        "twice(5, 10)",
    ]) {
        let result = relations
            .search_checked(query, &SearchConfig::default())
            .unwrap();
        assert!(result.complete);
        assert_eq!(result.answers.len(), 1);
        assert_eq!(result.answers[0].term, parse_term(expected).unwrap());
        relations.verify(&result.answers[0].derivation).unwrap();
    }
    let output = support::run(&[
        "infer",
        "examples/computed-bindings.soda",
        "--trace",
        "--color",
        "never",
    ]);
    assert!(output.status.success(), "{}", support::stderr(&output));
    let text = support::stdout(&output);
    assert!(text.contains("3 goals resolved"), "{text}");
    assert!(text.contains("16"), "{text}");
}

#[test]
fn certificates_cannot_omit_bindings_or_change_computed_results() {
    let relations = model(EXAMPLE);
    let answer = relations
        .search(
            &parse_term("twice(5, ?r)").unwrap(),
            &SearchConfig::default(),
        )
        .answers
        .remove(0);
    let mut missing = answer.derivation.clone();
    missing.substitution.remove("result");
    assert!(relations.verify(&missing).is_err());
    let mut extra = answer.derivation.clone();
    extra.substitution.insert("extra".into(), Term::Int(10));
    assert!(relations.verify(&extra).is_err());
    let mut wrong = answer.derivation.clone();
    wrong.substitution.insert("result".into(), Term::Int(11));
    wrong.conclusion = parse_term("twice(5, 11)").unwrap();
    assert!(
        relations
            .verify(&wrong)
            .unwrap_err()
            .contains("computed binding")
    );
    let mut open = answer.derivation;
    open.substitution
        .insert("n".into(), Term::Var("unknown".into()));
    open.conclusion = parse_term("twice(?unknown, 10)").unwrap();
    assert!(relations.verify(&open).is_err());
}

