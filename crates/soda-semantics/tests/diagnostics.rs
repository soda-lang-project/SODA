use soda_ast::{Rule, Term};
use soda_semantics::{Config, Engine, RuntimeErrorKind, Status};

#[test]
fn failures_keep_the_last_user_rule_without_collecting_a_full_trace() {
    let engine = Engine::new(vec![Rule {
        name: "boom".into(),
        lhs: Term::app("boom", vec![]),
        rhs: Term::app("div", vec![Term::Int(1), Term::Int(0)]),
        condition: None,
    }])
    .unwrap();
    let term = Term::app("boom", vec![]);

    for result in [
        engine.evaluate(&term, &Config::default()),
        engine.denote(&term, &Config::default()),
    ] {
        assert!(matches!(result.status, Status::Error(_)));
        assert!(result.trace.is_empty());
        assert_eq!(result.last_rule.as_deref(), Some("boom"));
    }
}

#[test]
fn guard_evaluation_does_not_replace_the_observable_rule_provenance() {
    let engine = Engine::new(vec![
        Rule {
            name: "outer".into(),
            lhs: Term::app("outer", vec![]),
            rhs: Term::Bool(true),
            condition: Some(Term::app("probe", vec![])),
        },
        Rule {
            name: "guard_only".into(),
            lhs: Term::app("probe", vec![]),
            rhs: Term::Bool(true),
            condition: None,
        },
    ])
    .unwrap();
    let result = engine.evaluate(&Term::app("outer", vec![]), &Config::default());

    assert_eq!(result.last_rule.as_deref(), Some("outer"));
    assert_eq!(result.steps, 2);
}

#[test]
fn runtime_causes_survive_nested_guard_context() {
    let engine = Engine::new(vec![
        Rule {
            name: "outer".into(),
            lhs: Term::app("outer", vec![]),
            rhs: Term::Bool(true),
            condition: Some(Term::app("inner", vec![])),
        },
        Rule {
            name: "inner".into(),
            lhs: Term::app("inner", vec![]),
            rhs: Term::Bool(true),
            condition: Some(Term::app("div", vec![Term::Int(1), Term::Int(0)])),
        },
    ])
    .unwrap();
    let result = engine.evaluate(&Term::app("outer", vec![]), &Config::default());
    let Status::Error(error) = result.status else {
        panic!("expected a runtime error");
    };

    assert_eq!(error.kind, RuntimeErrorKind::DivisionByZero);
    assert_eq!(error.guards, ["inner", "outer"]);
    assert_eq!(
        error.to_string(),
        "guard of outer: guard of inner: div by zero"
    );
    assert!(result.last_rule.is_none());
}
