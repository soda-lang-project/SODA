use soda_ast::{Rule, Term};
use soda_semantics::{Config, Engine, Status};

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn variable(name: &str) -> Term {
    Term::Var(name.into())
}

fn atom(name: &str) -> Term {
    app(name, vec![])
}

fn apply(function: Term, argument: Term) -> Term {
    app("apply", vec![function, argument])
}

fn rule(name: &str, lhs: Term, rhs: Term) -> Rule {
    Rule {
        name: name.into(),
        lhs,
        rhs,
        condition: None,
    }
}

fn combinators() -> Engine {
    let x = variable("x");
    let y = variable("y");
    let z = variable("z");

    Engine::new(vec![
        rule("identity", apply(atom("i"), x.clone()), x.clone()),
        rule(
            "constant",
            apply(apply(atom("k"), x.clone()), y.clone()),
            x.clone(),
        ),
        rule(
            "distribute",
            apply(apply(apply(atom("s"), x.clone()), y.clone()), z.clone()),
            apply(apply(x, z.clone()), apply(y, z)),
        ),
    ])
    .unwrap()
}

#[test]
fn unchanged_ski_rules_compute_identity_and_ignore_divergent_arguments() {
    let engine = combinators();
    let identity = apply(
        apply(apply(atom("s"), atom("k")), atom("k")),
        atom("answer"),
    );
    let result = engine.evaluate(&identity, &Config::default());

    assert_eq!(result.status, Status::NormalForm);
    assert_eq!(result.term, atom("answer"));
    let divergent = apply(
        apply(apply(atom("s"), atom("i")), atom("i")),
        apply(apply(atom("s"), atom("i")), atom("i")),
    );
    let constant = apply(apply(atom("k"), Term::Int(42)), divergent);
    let result = engine.evaluate(&constant, &Config::default());

    assert_eq!(result.status, Status::NormalForm);
    assert_eq!(result.term, Term::Int(42));
}

#[test]
fn custom_application_exposes_every_rule_and_both_argument_contexts() {
    let engine = Engine::new(vec![
        rule("first", apply(atom("i"), variable("x")), variable("x")),
        rule(
            "second",
            apply(atom("i"), variable("x")),
            atom("alternative"),
        ),
    ])
    .unwrap();
    let root = engine
        .successors(&apply(atom("i"), Term::Int(1)), &Config::default())
        .unwrap();

    assert_eq!(root.len(), 2);
    assert_eq!(
        root.iter()
            .map(|step| step.rule.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    let contextual = apply(
        atom("unknown"),
        app("add", vec![Term::Int(1), Term::Int(2)]),
    );
    let steps = engine.successors(&contextual, &Config::default()).unwrap();

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].path, vec![1]);
    assert_eq!(steps[0].after, Term::Int(3));
    assert_eq!(
        engine.denote(&contextual, &Config::default()).term,
        apply(atom("unknown"), Term::Int(3))
    );
    let both = apply(
        app("add", vec![Term::Int(1), Term::Int(2)]),
        app("mul", vec![Term::Int(3), Term::Int(4)]),
    );
    let steps = engine.successors(&both, &Config::default()).unwrap();

    assert_eq!(
        steps
            .iter()
            .map(|step| step.path.clone())
            .collect::<Vec<_>>(),
        vec![vec![0], vec![1]]
    );
}

#[test]
fn binder_application_keeps_primitive_priority_and_a_lazy_argument() {
    let engine = Engine::new(vec![rule(
        "legacy_catchall",
        apply(variable("function"), variable("argument")),
        atom("legacy"),
    )])
    .unwrap();
    let binder = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::Int(42)),
    };
    let term = apply(binder, app("div", vec![Term::Int(1), Term::Int(0)]));

    for result in [
        engine.evaluate(&term, &Config::default()),
        engine.denote(&term, &Config::default()),
    ] {
        assert_eq!(result.status, Status::NormalForm);
        assert_eq!(result.term, Term::Int(42));
    }

    let steps = engine.successors(&term, &Config::default()).unwrap();

    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].rule, "builtin:apply");
    assert_eq!(steps[0].after, Term::Int(42));
}

