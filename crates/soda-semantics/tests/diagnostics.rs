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

