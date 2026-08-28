use soda_ast::{
    Equation, InductionCase, InductionProof, Module, ProofStep, Signature, SignatureKind, SortDecl,
    Term,
};
use soda_proof::verify_inductions;

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn zero() -> Term {
    app("zero", vec![])
}

fn succ(term: Term) -> Term {
    app("succ", vec![term])
}

fn plus(lhs: Term, rhs: Term) -> Term {
    app("plus", vec![lhs, rhs])
}

fn step(name: &str, path: &[usize], bindings: &[(&str, Term)]) -> ProofStep {
    ProofStep {
        axiom: name.into(),
        reverse: false,
        path: path.into(),
        substitution: bindings
            .iter()
            .map(|(name, value)| ((*name).into(), value.clone()))
            .collect(),
    }
}

fn nat() -> Module {
    Module {
        sorts: vec![SortDecl {
            name: "Nat".into(),
            inductive: true,
        }],
        signatures: vec![
            Signature {
                name: "zero".into(),
                arguments: vec![],
                result: "Nat".into(),
                kind: SignatureKind::Constructor,
            },
            Signature {
                name: "succ".into(),
                arguments: vec!["Nat".into()],
                result: "Nat".into(),
                kind: SignatureKind::Constructor,
            },
            Signature {
                name: "plus".into(),
                arguments: vec!["Nat".into(), "Nat".into()],
                result: "Nat".into(),
                kind: SignatureKind::Function,
            },
        ],
        equations: vec![
            Equation {
                name: "plus0".into(),
                lhs: plus(zero(), var("y")),
                rhs: var("y"),
            },
            Equation {
                name: "plussucc".into(),
                lhs: plus(succ(var("x")), var("y")),
                rhs: succ(plus(var("x"), var("y"))),
            },
        ],
        inductions: vec![InductionProof {
            name: "rightzero".into(),
            variable: "n".into(),
            sort: "Nat".into(),
            lhs: plus(var("n"), zero()),
            rhs: var("n"),
            cases: vec![
                InductionCase {
                    constructor: "zero".into(),
                    arguments: vec![],
                    steps: vec![step("plus0", &[], &[("y", zero())])],
                },
                InductionCase {
                    constructor: "succ".into(),
                    arguments: vec!["k".into()],
                    steps: vec![
                        step("plussucc", &[], &[("x", var("k")), ("y", zero())]),
                        step("ih_k", &[0], &[("k", var("k"))]),
                    ],
                },
            ],
        }],
        ..Module::default()
    }
}

#[test]
fn proves_right_zero_by_structural_induction() {
    let checked = verify_inductions(&nat()).unwrap();

    assert_eq!(checked[0].cases, 2);
    assert_eq!(checked[0].steps, 3);
}

