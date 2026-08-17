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

#[test]
fn repeated_variables_are_constraints() {
    let equation = Equation {
        name: "same".into(),
        lhs: app("pair", vec![var("x"), var("x")]),
        rhs: Term::Bool(true),
    };
    let proof = Proof {
        name: "false".into(),
        lhs: app("pair", vec![Term::Int(1), Term::Int(2)]),
        rhs: Term::Bool(true),
        steps: vec![ProofStep {
            axiom: "same".into(),
            reverse: false,
            path: vec![],
            substitution: Substitution::from([("x".into(), Term::Int(1))]),
        }],
    };

    assert!(verify_proof(&[equation], &proof).is_err());
}

#[test]
fn rejects_unknown_axioms_shadowing_and_self_reference() {
    let mut proof = certificate();
    proof.steps[0].axiom = proof.name.clone();

    assert!(verify_proof(&[axiom()], &proof).is_err());
    assert!(verify_proof(&[axiom(), axiom()], &certificate()).is_err());
    let mut proof = certificate();
    proof.name = "identity".into();

    assert!(verify_proof(&[axiom()], &proof).is_err());
    let module = Module {
        equations: vec![axiom()],
        proofs: vec![certificate(), certificate()],
        ..Module::default()
    };

    assert!(verify_module(&module).is_err());
}

#[test]
fn empty_certificate_proves_only_reflexivity() {
    let mut proof = certificate();
    proof.steps.clear();

    assert!(verify_proof(&[], &proof).is_err());
    proof.rhs = proof.lhs.clone();

    assert!(verify_proof(&[], &proof).is_ok());
}

#[test]
fn validates_rule_and_condition_scope() {
    let mut module = Module {
        rules: vec![Rule {
            name: "unsafe".into(),
            lhs: app("f", vec![var("x")]),
            rhs: var("y"),
            condition: None,
        }],
        ..Module::default()
    };

    assert!(validate_module(&module).is_err());
    module.rules[0].rhs = var("x");
    module.rules[0].condition = Some(var("y"));

    assert!(validate_module(&module).is_err());
    module.rules[0].condition = Some(var("x"));

    assert!(validate_module(&module).is_ok());
    module.rules[0].lhs = var("x");

    assert!(validate_module(&module).is_err());
}

#[test]
fn termination_analysis_is_conservative_about_duplication_and_conditions() {
    let mut rule = Rule {
        name: "reduce".into(),
        lhs: app("f", vec![var("x")]),
        rhs: var("x"),
        condition: None,
    };

    assert!(analyze_termination(&[rule.clone()]).proven);
    rule.condition = Some(Term::Bool(true));

    assert!(!analyze_termination(&[rule.clone()]).proven);
    rule.condition = None;
    rule.lhs = app("f", vec![app("g", vec![var("x")])]);
    rule.rhs = app("pair", vec![var("x"), var("x")]);

    assert!(!analyze_termination(&[rule]).proven);
}

#[test]
fn unification_composes_bindings_and_checks_occurs() {
    let left = app("pair", vec![var("x"), var("y")]);
    let right = app("pair", vec![var("y"), Term::Int(3)]);
    let substitution = unify(&left, &right).unwrap();

    assert_eq!(
        left.substitute(&substitution),
        right.substitute(&substitution)
    );
    for value in substitution.values() {
        assert_eq!(value.substitute(&substitution), *value);
    }

    assert!(unify(&var("x"), &app("f", vec![var("x")])).is_none());
    assert!(unify(&app("f", vec![]), &app("f", vec![Term::Int(1)])).is_none());
    assert!(unify(&Term::Int(1), &Term::Int(2)).is_none());
}

#[test]
fn clause_names_share_the_declaration_namespace_and_existentials_are_allowed() {
    let clause = Clause {
        name: "exists".into(),
        conclusion: app("exists", vec![]),
        premises: vec![app("p", vec![var("witness")])],
    };
    let mut module = Module {
        clauses: vec![clause.clone()],
        ..Module::default()
    };

    assert!(validate_module(&module).is_ok());
    module.clauses.push(clause);

    assert!(validate_module(&module).is_err());
    module.clauses.pop();
    module.equations.push(Equation {
        name: "exists".into(),
        lhs: Term::Int(1),
        rhs: Term::Int(1),
    });

    assert!(validate_module(&module).is_err());
    module.equations.clear();
    module.clauses[0].conclusion = var("x");

    assert!(validate_module(&module).is_err());
    module.clauses[0].conclusion = app("exists", vec![]);
    module.queries.push(app("", vec![]));

    assert!(validate_module(&module).is_err());
}

#[test]
fn rejects_literal_rule_heads() {
    for lhs in [Term::Int(0), Term::Bool(false)] {
        let module = Module {
            rules: vec![Rule {
                name: "literal".into(),
                lhs,
                rhs: Term::Int(1),
                condition: None,
            }],
            ..Module::default()
        };

        assert!(validate_module(&module).is_err());
    }
}

#[test]
fn term_depth_limit_is_conservative_and_checked_iteratively() {
    fn nested(depth: usize) -> Term {
        (0..depth).fold(Term::Int(0), |term, _| app("f", vec![term]))
    }
    let module = Module {
        evaluations: vec![nested(256)],
        ..Module::default()
    };

    assert!(validate_module(&module).is_ok());
    let module = Module {
        queries: vec![nested(257)],
        ..Module::default()
    };
    let error = validate_module(&module).unwrap_err();

    assert!(error.message.contains("depth limit"));
    let equation = Equation {
        name: "deep".into(),
        lhs: nested(2048),
        rhs: Term::Int(0),
    };
    let proof = Proof {
        name: "reflexive".into(),
        lhs: Term::Int(0),
        rhs: Term::Int(0),
        steps: vec![],
    };

    assert!(verify_proof(&[equation], &proof).is_err());
}

#[test]
fn binder_unification_and_certificates_preserve_scope() {
    let name = |n: &str| Term::Name(n.into());
    let binder = |n: &str, body| Term::Binder {
        parameter: n.into(),
        sort: "Expr".into(),
        body: Box::new(body),
    };
    let left = binder("x", name("x"));
    let right = binder("y", name("y"));

    assert!(unify(&left, &right).is_some());
    assert!(unify(&binder("x", var("body")), &right).is_none());
    assert!(
        verify_proof(
            &[],
            &Proof {
                name: "alpha".into(),
                lhs: left.clone(),
                rhs: right,
                steps: vec![]
            }
        )
        .is_ok()
    );
    let axiom = Equation {
        name: "free_names".into(),
        lhs: name("x"),
        rhs: name("z"),
    };
    let forged = Proof {
        name: "escape".into(),
        lhs: left,
        rhs: binder("x", name("z")),
        steps: vec![ProofStep {
            axiom: "free_names".into(),
            reverse: false,
            path: vec![0],
            substitution: Substitution::new(),
        }],
    };

    assert!(verify_proof(&[axiom], &forged).is_err());
}
