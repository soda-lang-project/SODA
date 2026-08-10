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

#[test]
fn in_place_reduction_preserves_partial_results_limits_and_trace_positions() {
    let engine = Engine::new(vec![rule(
        "grow",
        app("grow", vec![]),
        app("box", vec![int(1)]),
        None,
    )])
    .unwrap();
    let initial = app(
        "data",
        vec![app("add", vec![int(1), int(2)]), app("grow", vec![])],
    );

    for strategy in [Strategy::Outermost, Strategy::Innermost] {
        let config = Config {
            max_depth: 1,
            trace: true,
            strategy,
            ..Config::default()
        };

        let invalid = engine.evaluate(&initial, &config);

        assert_eq!(invalid.term, initial);
        assert_eq!(invalid.steps, 0);
        let shallow = app("data", vec![int(3), app("grow", vec![])]);
        let result = engine.evaluate(&shallow, &config);

        assert_eq!(result.status, Status::Limit(Limit::Depth));
        assert_eq!(result.steps, 1);
        assert_eq!(result.trace[0].path, [1]);
        assert_eq!(
            result.term,
            shallow.replace(&[1], app("box", vec![int(1)])).unwrap()
        );

        let failing = app(
            "data",
            vec![app("grow", vec![]), app("div", vec![int(1), int(0)])],
        );
        let result = engine.evaluate(
            &failing,
            &Config {
                strategy,
                trace: true,
                ..Config::default()
            },
        );

        assert!(matches!(result.status, Status::Error(_)));
        assert_eq!(
            result.term,
            failing.replace(&[0], app("box", vec![int(1)])).unwrap()
        );
        let exhausted = engine.evaluate(
            &failing,
            &Config {
                strategy,
                fuel: 0,
                ..Config::default()
            },
        );

        assert_eq!(exhausted.status, Status::Limit(Limit::Fuel));
        assert_eq!(exhausted.term, failing);
    }
}

