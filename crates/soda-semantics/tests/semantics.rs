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

#[test]
fn fuel_depth_and_divergent_guards_are_explicit_limits() {
    let engine = Engine::new(vec![
        rule("loop", app("loop", vec![]), app("loop", vec![]), None),
        rule(
            "guarded",
            app("guarded", vec![]),
            int(1),
            Some(app("loop", vec![])),
        ),
        rule(
            "grow",
            app("grow", vec![]),
            app("box", vec![app("grow", vec![])]),
            None,
        ),
    ])
    .unwrap();
    let config = Config {
        fuel: 8,
        ..Config::default()
    };

    for name in ["loop", "guarded"] {
        let result = engine.evaluate(&app(name, vec![]), &config);

        assert_eq!(result.status, Status::Limit(Limit::Fuel));
        assert_eq!(result.steps, 8);
        assert_eq!(
            engine.denote(&app(name, vec![]), &config).status,
            Status::Limit(Limit::Fuel)
        );
    }

    let config = Config {
        fuel: 100,
        max_depth: 4,
        ..Config::default()
    };

    assert_eq!(
        engine.evaluate(&app("grow", vec![]), &config).status,
        Status::Limit(Limit::Depth)
    );
    assert_eq!(
        engine.denote(&app("grow", vec![]), &config).status,
        Status::Limit(Limit::Depth)
    );
    let engine = Engine::new(vec![rule(
        "self_guard",
        app("f", vec![]),
        int(1),
        Some(app("f", vec![])),
    )])
    .unwrap();

    assert_eq!(
        engine.evaluate(&app("f", vec![]), &config).status,
        Status::Limit(Limit::Depth)
    );
    let unbounded_request = Config {
        max_depth: usize::MAX,
        ..Config::default()
    };

    assert_eq!(
        engine
            .evaluate(&app("f", vec![]), &unbounded_request)
            .status,
        Status::Limit(Limit::Depth)
    );
}

#[test]
fn false_and_undecidable_guards_are_stuck_and_alternatives_work() {
    for guard in [Term::Bool(false), app("unknown", vec![]), var("x")] {
        let engine = Engine::new(vec![rule(
            "guard",
            app("f", vec![var("x")]),
            int(1),
            Some(guard),
        )])
        .unwrap();

        assert!(matches!(
            evaluate(&engine, &app("f", vec![int(2)])).status,
            Status::Stuck(_)
        ));
    }

    let engine = Engine::new(vec![
        rule("reject", app("f", vec![]), int(1), Some(Term::Bool(false))),
        rule("fallback", app("f", vec![]), int(2), None),
    ])
    .unwrap();

    assert_eq!(evaluate(&engine, &app("f", vec![])).term, int(2));
}

#[test]
fn structural_equality_normalizes_defined_children() {
    let engine = Engine::new(vec![rule(
        "key",
        app("key", vec![]),
        app("a", vec![]),
        None,
    )])
    .unwrap();
    let term = app(
        "eq",
        vec![
            app("pair", vec![app("key", vec![]), int(2)]),
            app("pair", vec![app("a", vec![]), int(2)]),
        ],
    );

    assert_eq!(evaluate(&engine, &term).term, Term::Bool(true));
    assert_eq!(
        engine.denote(&term, &Config::default()).term,
        Term::Bool(true)
    );
    let open = app("eq", vec![var("x"), var("x")]);

    assert!(matches!(evaluate(&engine, &open).status, Status::Stuck(_)));
    assert!(matches!(
        engine.denote(&open, &Config::default()).status,
        Status::Stuck(_)
    ));
}

#[test]
fn trace_replays_and_repeated_variables_match_consistently() {
    let engine = Engine::new(vec![rule(
        "same",
        app("same", vec![var("x"), var("x")]),
        Term::Bool(true),
        None,
    )])
    .unwrap();
    let input = app(
        "pair",
        vec![
            app("same", vec![int(1), int(1)]),
            app("add", vec![int(2), int(3)]),
        ],
    );

    for strategy in [Strategy::Outermost, Strategy::Innermost] {
        let config = Config {
            trace: true,
            strategy,
            ..Config::default()
        };
        let result = engine.evaluate(&input, &config);

        assert_eq!(result.status, Status::NormalForm);
        let mut replay = input.clone();

        for step in &result.trace {
            assert_eq!(replay.at(&step.path), Some(&step.before));
            replay = replay.replace(&step.path, step.after.clone()).unwrap();
        }

        assert_eq!(replay, result.term);
        assert_eq!(
            result
                .trace
                .iter()
                .map(|s| s.rule.as_str())
                .collect::<Vec<_>>(),
            ["same", "builtin:add"]
        );
    }

    assert!(matches!(
        evaluate(&engine, &app("same", vec![int(1), int(2)])).status,
        Status::Stuck(_)
    ));
}

#[test]
fn strategies_are_observably_distinct_and_zero_fuel_can_accept_values() {
    let engine = Engine::new(vec![
        rule("erase", app("erase", vec![var("x")]), int(7), None),
        rule("loop", app("loop", vec![]), app("loop", vec![]), None),
    ])
    .unwrap();
    let input = app("erase", vec![app("loop", vec![])]);
    let config = Config {
        fuel: 5,
        ..Config::default()
    };

    assert_eq!(engine.evaluate(&input, &config).term, int(7));
    assert_eq!(
        engine
            .evaluate(
                &input,
                &Config {
                    strategy: Strategy::Innermost,
                    ..config.clone()
                }
            )
            .status,
        Status::Limit(Limit::Fuel)
    );
    assert_eq!(
        engine.denote(&input, &config).status,
        Status::Limit(Limit::Fuel)
    );
    assert_eq!(
        engine
            .evaluate(&int(1), &Config { fuel: 0, ..config })
            .status,
        Status::NormalForm
    );
}

#[test]
fn compositional_guards_do_not_depend_on_operational_strategy() {
    let engine = Engine::new(vec![
        rule(
            "erase",
            app("erase", vec![var("x")]),
            Term::Bool(true),
            None,
        ),
        rule("loop", app("loop", vec![]), app("loop", vec![]), None),
        rule(
            "guard",
            app("guard", vec![]),
            int(1),
            Some(app("erase", vec![app("loop", vec![])])),
        ),
    ])
    .unwrap();
    let term = app("guard", vec![]);

    for strategy in [Strategy::Outermost, Strategy::Innermost] {
        let config = Config {
            fuel: 5,
            strategy,
            ..Config::default()
        };

        assert_eq!(
            engine.denote(&term, &config).status,
            Status::Limit(Limit::Fuel)
        );
    }
}

