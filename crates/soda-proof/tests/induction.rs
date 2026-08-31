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

#[test]
fn earlier_inductive_theorems_are_reusable_lemmas() {
    let mut module = nat();
    let mut second = module.inductions[0].clone();
    second.name = "rightzero_again".into();
    second.cases[0].steps = vec![step("rightzero", &[], &[("n", zero())])];
    second.cases[1].steps = vec![step("rightzero", &[], &[("n", succ(var("k")))])];
    module.inductions.push(second);

    assert_eq!(verify_inductions(&module).unwrap().len(), 2);
}

#[test]
fn rejects_missing_duplicate_and_unknown_cases() {
    let mut module = nat();
    module.inductions[0].cases.pop();

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("cover each constructor")
    );
    let mut module = nat();
    let duplicate = module.inductions[0].cases[0].clone();
    module.inductions[0].cases.push(duplicate);

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("cover each constructor")
    );
    let mut module = nat();
    module.inductions[0].cases[0].constructor = "bogus".into();

    assert!(verify_inductions(&module).is_err());
}

#[test]
fn rejects_nonfresh_wrong_arity_and_duplicate_fields() {
    for fields in [vec!["n".into()], vec![], vec!["k".into(), "k".into()]] {
        let mut module = nat();
        module.inductions[0].cases[1].arguments = fields;

        assert!(
            verify_inductions(&module)
                .unwrap_err()
                .to_string()
                .contains("fresh, distinct")
        );
    }
}

#[test]
fn rejects_non_datatype_and_absent_induction_variables() {
    let mut module = nat();
    module.sorts[0].inductive = false;

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("not an inductive")
    );
    let mut module = nat();
    module.inductions[0].variable = "missing".into();

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("must occur")
    );
}

#[test]
fn annotation_constrains_the_theorem_variable_not_an_isolated_witness() {
    let mut module = nat();
    module.inductions[0].lhs = app("add", vec![var("n"), Term::Int(0)]);
    module.inductions[0].rhs = var("n");

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("sort mismatch")
    );
}

#[test]
fn induction_hypotheses_cannot_be_instantiated_at_larger_terms() {
    let mut module = nat();
    module.inductions[0].cases[1].steps[1]
        .substitution
        .insert("k".into(), succ(var("k")));

    assert!(
        verify_inductions(&module)
            .unwrap_err()
            .to_string()
            .contains("remain anchored")
    );
}

