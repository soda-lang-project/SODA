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

#[test]
fn symmetry_and_open_terms_are_sound() {
    let mut proof = certificate();
    std::mem::swap(&mut proof.lhs, &mut proof.rhs);
    proof.steps[0].reverse = true;

    assert!(verify_proof(&[axiom()], &proof).is_ok());
    let equation = axiom();
    proof.lhs = var("y");
    proof.rhs = app("add", vec![var("y"), Term::Int(0)]);
    proof.steps[0].path.clear();
    proof.steps[0].substitution.insert("x".into(), var("y"));

    assert!(verify_proof(&[equation], &proof).is_ok());
}

#[test]
fn rejects_bad_positions_without_panicking() {
    for path in [vec![1], vec![0, 2], vec![0, 0, 0], vec![usize::MAX]] {
        let mut proof = certificate();
        proof.steps[0].path = path;

        assert!(verify_proof(&[axiom()], &proof).is_err());
    }
}

#[test]
fn rejects_wrong_instance_and_wrong_conclusion() {
    let mut proof = certificate();
    proof.steps[0].substitution.insert("x".into(), Term::Int(8));

    assert!(verify_proof(&[axiom()], &proof).is_err());
    let mut proof = certificate();
    proof.rhs = Term::Bool(false);

    assert!(verify_proof(&[axiom()], &proof).is_err());
}

#[test]
fn rejects_missing_and_extraneous_bindings() {
    let mut proof = certificate();
    proof.steps[0].substitution.clear();

    assert!(verify_proof(&[axiom()], &proof).is_err());
    let mut proof = certificate();
    proof.steps[0]
        .substitution
        .insert("extra".into(), Term::Int(9));

    assert!(verify_proof(&[axiom()], &proof).is_err());
}

#[test]
fn binds_variables_on_both_sides_even_in_reverse() {
    let equation = Equation {
        name: "assumption".into(),
        lhs: var("x"),
        rhs: var("y"),
    };
    let mut proof = Proof {
        name: "example".into(),
        lhs: Term::Int(1),
        rhs: Term::Int(2),
        steps: vec![ProofStep {
            axiom: "assumption".into(),
            reverse: true,
            path: vec![],
            substitution: Substitution::from([("y".into(), Term::Int(1))]),
        }],
    };

    assert!(verify_proof(std::slice::from_ref(&equation), &proof).is_err());
    proof.steps[0].substitution.insert("x".into(), Term::Int(2));

    assert!(verify_proof(&[equation], &proof).is_ok());
}

