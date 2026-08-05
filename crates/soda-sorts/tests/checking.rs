use soda_ast::{
    Clause, JudgmentDecl, JudgmentKind, Module, Rule, Signature, SignatureKind, SortDecl, Term,
};
use soda_ast::{Equation, Proof, ProofStep};
use soda_sorts::{Sort, TypeChecker, validate_module};

fn signature(name: &str, args: &[&str], result: &str) -> Signature {
    Signature {
        name: name.into(),
        arguments: args.iter().map(|s| (*s).into()).collect(),
        result: result.into(),
        kind: SignatureKind::Function,
    }
}

fn typed() -> Module {
    Module {
        sorts: vec![SortDecl {
            name: "Expr".into(),
            inductive: false,
        }],
        signatures: vec![
            signature("num", &["Int"], "Expr"),
            signature("pair", &["Expr", "Expr"], "Expr"),
        ],
        judgments: vec![JudgmentDecl {
            name: "eval".into(),
            arguments: vec!["Expr".into(), "Int".into()],
            kind: JudgmentKind::Evaluation,
        }],
        ..Module::default()
    }
}

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

#[test]
fn contradictory_variables_are_rejected_across_rule_sides_and_conditions() {
    let mut module = typed();
    module.rules.push(Rule {
        name: "bad".into(),
        lhs: app("num", vec![var("x")]),
        rhs: app("num", vec![Term::Int(0)]),
        condition: Some(var("x")),
    });

    assert!(
        validate_module(&module)
            .unwrap_err()
            .to_string()
            .contains("sort mismatch")
    );
}

#[test]
fn schema_errors_are_not_treated_as_untyped_fallbacks() {
    for bad in [
        signature("bad", &["Missing"], "Int"),
        signature("add", &[], "Int"),
    ] {
        let module = Module {
            signatures: vec![bad],
            ..Module::default()
        };

        assert!(TypeChecker::new(&module).is_err());
    }

    let mut module = typed();
    module.sorts.push(module.sorts[0].clone());

    assert!(validate_module(&module).is_err());
}

#[test]
fn judgment_sorts_are_shared_across_all_premises() {
    let mut module = typed();
    module.clauses.push(Clause {
        name: "bad".into(),
        conclusion: app("eval", vec![app("num", vec![var("x")]), var("x")]),
        premises: vec![app("eval", vec![var("x"), Term::Int(0)])],
    });

    assert!(validate_module(&module).is_err());
    module.clauses[0].premises.clear();

    assert!(validate_module(&module).is_ok());
}

#[test]
fn judgments_cannot_be_data_and_data_cannot_be_judgments() {
    let module = typed();
    let checker = TypeChecker::new(&module).unwrap();
    let goal = app("eval", vec![app("num", vec![Term::Int(1)]), var("n")]);

    assert!(checker.check_judgment(&goal).is_ok());
    assert!(checker.check_term(&goal).is_err());
    assert!(
        checker
            .check_judgment(&app("num", vec![Term::Int(1)]))
            .is_err()
    );
    assert!(checker.check_term(&app("pair", vec![])).is_err());
}

#[test]
fn lexical_scope_shadowing_and_function_application_are_checked() {
    let checker = TypeChecker::new(&typed()).unwrap();
    let identity = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::Name("x".into())),
    };

    assert_eq!(
        checker
            .check_term(&app("apply", vec![identity.clone(), Term::Int(5)]))
            .unwrap(),
        Sort::Named("Int".into())
    );
    assert!(
        checker
            .check_term(&app("apply", vec![identity, Term::Bool(true)]))
            .is_err()
    );
    assert!(checker.check_term(&Term::Name("x".into())).is_err());
    let nested = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::Binder {
            parameter: "x".into(),
            sort: "Bool".into(),
            body: Box::new(Term::Name("x".into())),
        }),
    };

    assert_eq!(
        checker.check_term(&nested).unwrap().to_string(),
        "Fn(Int, Fn(Bool, Bool))"
    );
}

#[test]
fn polymorphic_builtins_unify_branches_and_substitution() {
    let checker = TypeChecker::new(&typed()).unwrap();

    assert!(
        checker
            .check_term(&app(
                "if",
                vec![Term::Bool(true), Term::Int(1), Term::Bool(false)]
            ))
            .is_err()
    );
    let expr = app("num", vec![Term::Int(1)]);

    assert_eq!(
        checker
            .check_term(&app("eq", vec![expr.clone(), expr]))
            .unwrap(),
        Sort::Named("Bool".into())
    );
    let substitute = app(
        "subst",
        vec![
            app("add", vec![Term::Name("x".into()), Term::Int(1)]),
            Term::Name("x".into()),
            Term::Int(4),
        ],
    );

    assert_eq!(
        checker.check_term(&substitute).unwrap(),
        Sort::Named("Int".into())
    );
}

#[test]
fn nested_function_sorts_and_occurs_check() {
    let checker = TypeChecker::new(&typed()).unwrap();

    assert_eq!(
        checker
            .parse_sort("Fn(Fn(Int, Bool), Int)")
            .unwrap()
            .to_string(),
        "Fn(Fn(Int, Bool), Int)"
    );
    assert!(checker.parse_sort("Fn(Int, Bool, Int)").is_err());
    assert!(
        checker
            .check_term(&app("apply", vec![var("f"), var("f")]))
            .unwrap_err()
            .to_string()
            .contains("infinite")
    );
}

#[test]
fn inductive_constructors_remain_free_and_strictly_positive() {
    let mut module = Module {
        sorts: vec![SortDecl {
            name: "Nat".into(),
            inductive: true,
        }],
        signatures: vec![Signature {
            name: "zero".into(),
            arguments: vec![],
            result: "Nat".into(),
            kind: SignatureKind::Constructor,
        }],
        ..Module::default()
    };
    module.rules.push(Rule {
        name: "bad".into(),
        lhs: app("zero", vec![]),
        rhs: app("zero", vec![]),
        condition: None,
    });

    assert!(validate_module(&module).is_err());
    module.rules.clear();
    module.signatures[0].arguments.push("Fn(Nat, Nat)".into());

    assert!(validate_module(&module).is_err());
}

