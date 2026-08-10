use soda_ast::{Rule, Term};
use soda_semantics::{Config, Engine, Limit, Status, Strategy};

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn int(value: i64) -> Term {
    Term::Int(value)
}

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn rule(name: &str, lhs: Term, rhs: Term, condition: Option<Term>) -> Rule {
    Rule {
        name: name.into(),
        lhs,
        rhs,
        condition,
    }
}

fn evaluate(engine: &Engine, term: &Term) -> soda_semantics::Evaluation {
    engine.evaluate(term, &Config::default())
}

#[test]
fn indexed_rules_preserve_order_arity_and_guard_fuel() {
    let engine = Engine::new(vec![
        rule("wrong_arity", app("f", vec![]), int(99), None),
        rule("unrelated", app("g", vec![var("x")]), int(99), None),
        rule(
            "disabled",
            app("f", vec![var("x")]),
            int(99),
            Some(app("eq", vec![int(1), int(2)])),
        ),
        rule("first", app("f", vec![var("x")]), var("x"), None),
        rule("second", app("f", vec![var("x")]), int(99), None),
    ])
    .unwrap();
    let term = app("f", vec![int(7)]);
    let config = Config {
        trace: true,
        ..Config::default()
    };

    for result in [
        engine.evaluate(&term, &config),
        engine.denote(&term, &config),
    ] {
        assert_eq!(result.term, int(7));
        assert_eq!(result.steps, 2);
        assert_eq!(result.trace.len(), 1);
        assert_eq!(result.trace[0].rule, "first");
    }

    let steps = engine.successors(&term, &config).unwrap();

    assert_eq!(
        steps
            .iter()
            .map(|step| step.rule.as_str())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
}

