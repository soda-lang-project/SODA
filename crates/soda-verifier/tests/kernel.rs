use soda_ast::{Clause, Equation, Module, Proof, ProofStep, Rule, Substitution, Term};
use soda_verifier::{analyze_termination, unify, validate_module, verify_module, verify_proof};

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn axiom() -> Equation {
    Equation {
        name: "identity".into(),
        lhs: app("add", vec![var("x"), Term::Int(0)]),
        rhs: var("x"),
    }
}

fn certificate() -> Proof {
    Proof {
        name: "example".into(),
        lhs: app("box", vec![app("add", vec![Term::Int(7), Term::Int(0)])]),
        rhs: app("box", vec![Term::Int(7)]),
        steps: vec![ProofStep {
            axiom: "identity".into(),
            reverse: false,
            path: vec![0],
            substitution: Substitution::from([("x".into(), Term::Int(7))]),
        }],
    }
}

#[test]
fn checks_substitution_and_congruence() {
    let result = verify_proof(&[axiom()], &certificate()).unwrap();

    assert_eq!(result.steps_checked, 1);
}

