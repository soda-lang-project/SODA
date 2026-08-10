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

#[test]
fn factorial_agrees_across_independent_interpreters() {
    let x = var("n");
    let engine = Engine::new(vec![rule(
        "factorial",
        app("fact", vec![x.clone()]),
        app(
            "if",
            vec![
                app("eq", vec![x.clone(), int(0)]),
                int(1),
                app(
                    "mul",
                    vec![x.clone(), app("fact", vec![app("sub", vec![x, int(1)])])],
                ),
            ],
        ),
        None,
    )])
    .unwrap();

    for n in 0..8 {
        let term = app("fact", vec![int(n)]);
        let expected: i64 = (1..=n).product();
        let operational = evaluate(&engine, &term);
        let denotational = engine.denote(&term, &Config::default());

        assert_eq!(operational.status, Status::NormalForm);
        assert_eq!(denotational.status, Status::NormalForm);
        assert_eq!(operational.term, int(expected));
        assert_eq!(denotational.term, int(expected));
    }
}

#[test]
fn all_builtins_and_checked_arithmetic() {
    let engine = Engine::new(vec![]).unwrap();

    for (name, a, b, expected) in [
        ("add", 8, 3, 11),
        ("sub", 8, 3, 5),
        ("mul", 8, 3, 24),
        ("div", 8, 3, 2),
        ("mod", 8, 3, 2),
    ] {
        assert_eq!(
            evaluate(&engine, &app(name, vec![int(a), int(b)])).term,
            int(expected)
        );
    }

    for name in ["lt", "le", "gt", "ge", "eq"] {
        assert_eq!(
            evaluate(&engine, &app(name, vec![int(2), int(2)])).term,
            Term::Bool(matches!(name, "le" | "ge" | "eq"))
        );
    }

    for (name, args, expected) in [
        ("and", vec![Term::Bool(true), Term::Bool(false)], false),
        ("or", vec![Term::Bool(true), Term::Bool(false)], true),
        ("not", vec![Term::Bool(false)], true),
    ] {
        assert_eq!(
            evaluate(&engine, &app(name, args)).term,
            Term::Bool(expected)
        );
    }

    for term in [
        app("div", vec![int(1), int(0)]),
        app("mod", vec![int(1), int(0)]),
        app("div", vec![int(i64::MIN), int(-1)]),
        app("mod", vec![int(i64::MIN), int(-1)]),
        app("add", vec![int(i64::MAX), int(1)]),
        app("sub", vec![int(i64::MIN), int(1)]),
        app("mul", vec![int(i64::MAX), int(2)]),
        app("add", vec![Term::Bool(true), int(1)]),
        app("not", vec![]),
        app("if", vec![int(1), int(2), int(3)]),
    ] {
        assert!(
            matches!(evaluate(&engine, &term).status, Status::Error(_)),
            "{term}"
        );
    }
}

#[test]
fn laziness_preserves_ignored_divergent_and_erroneous_branches() {
    let engine = Engine::new(vec![rule(
        "loop",
        app("loop", vec![]),
        app("loop", vec![]),
        None,
    )])
    .unwrap();

    for strategy in [Strategy::Outermost, Strategy::Innermost] {
        let config = Config {
            strategy,
            fuel: 10,
            ..Config::default()
        };
        let term = app("if", vec![Term::Bool(true), int(42), app("loop", vec![])]);

        assert_eq!(engine.evaluate(&term, &config).term, int(42));
        assert_eq!(engine.denote(&term, &config).term, int(42));
        let term = app(
            "if",
            vec![Term::Bool(false), app("div", vec![int(1), int(0)]), int(9)],
        );

        assert_eq!(engine.evaluate(&term, &config).term, int(9));
    }
}

