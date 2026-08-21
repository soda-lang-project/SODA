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

#[test]
fn typed_derivation_rejects_boolean_substitution_for_integer_metavariable() {
    let relations = Relations::from_module(&typed()).unwrap();
    let forged = Derivation {
        rule: "any_int".into(),
        conclusion: app("p", vec![Term::Bool(true)]),
        substitution: Substitution::from([("x".into(), Term::Bool(true))]),
        premises: vec![],
    };

    assert!(relations.verify(&forged).is_err());
    let valid = Derivation {
        rule: "any_int".into(),
        conclusion: app("p", vec![Term::Int(3)]),
        substitution: Substitution::from([("x".into(), Term::Int(3))]),
        premises: vec![],
    };
    relations.verify(&valid).unwrap();
}

#[test]
fn checked_search_reports_invalid_query_and_legacy_search_is_incomplete() {
    let relations = Relations::from_module(&typed()).unwrap();

    for query in [
        app("p", vec![Term::Bool(true)]),
        app("p", vec![]),
        app("unknown", vec![Term::Int(3)]),
    ] {
        assert!(
            relations
                .search_checked(&query, &SearchConfig::default())
                .is_err()
        );
        let result = relations.search(&query, &SearchConfig::default());

        assert!(!result.complete);
        assert!(result.answers.is_empty());
        assert_eq!(result.attempts, 0);
    }

    for query in [app("p", vec![Term::Int(7)]), app("p", vec![var("answer")])] {
        let result = relations
            .search_checked(&query, &SearchConfig::default())
            .unwrap();

        support::assert_single_verified_answer(&relations, &result);
    }
}

#[test]
fn typed_schema_rejects_invalid_definitions_and_infers_branch_bindings() {
    let mut invalid = typed();
    invalid.clauses[0].conclusion = app("p", vec![Term::Bool(true)]);

    assert!(Relations::from_module(&invalid).is_err());
    let mut module = typed();
    module.judgments.push(judgment("source", &["Int"]));
    module.clauses[0].premises = vec![app("source", vec![var("x")])];
    module
        .clauses
        .push(clause("three", app("source", vec![Term::Int(3)]), vec![]));
    let relations = Relations::from_module(&module).unwrap();
    let result = relations
        .search_checked(&app("p", vec![var("answer")]), &SearchConfig::default())
        .unwrap();

    assert!(result.complete);
    assert_eq!(result.answers[0].bindings["answer"], Term::Int(3));
    relations.verify(&result.answers[0].derivation).unwrap();
}

