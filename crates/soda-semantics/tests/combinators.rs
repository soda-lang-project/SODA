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

