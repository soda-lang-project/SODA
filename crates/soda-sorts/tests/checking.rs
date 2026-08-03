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

