mod support;

use soda_ast::{Clause, JudgmentDecl, JudgmentKind, Module, Substitution, Term};
use soda_relations::{Derivation, Relations, SearchConfig};

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn judgment(name: &str, sorts: &[&str]) -> JudgmentDecl {
    JudgmentDecl {
        name: name.into(),
        arguments: sorts.iter().map(|sort| (*sort).into()).collect(),
        kind: JudgmentKind::Relation,
    }
}

fn clause(name: &str, conclusion: Term, premises: Vec<Term>) -> Clause {
    Clause {
        name: name.into(),
        conclusion,
        premises,
    }
}

fn typed() -> Module {
    Module {
        judgments: vec![judgment("p", &["Int"])],
        clauses: vec![clause("any_int", app("p", vec![var("x")]), vec![])],
        ..Module::default()
    }
}

#[test]
fn runtime_binders_cannot_bypass_legacy_schema_checking() {
    let module = Module {
        clauses: vec![clause("anything", app("p", vec![var("x")]), vec![])],
        ..Module::default()
    };
    let relations = Relations::from_module(&module).unwrap();
    let binding = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::Name("x".into())),
    };
    let query = app("p", vec![binding.clone()]);

    assert!(
        relations
            .search_checked(&query, &SearchConfig::default())
            .is_err()
    );
    let forged = Derivation {
        rule: "anything".into(),
        conclusion: query,
        substitution: [("x".into(), binding)].into(),
        premises: vec![],
    };

    assert!(relations.verify(&forged).is_err());
}

