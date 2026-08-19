mod support;

use soda_ast::{Clause, ClauseDetail, Module, Rule, Substitution, Term};
use soda_relations::{Derivation, Relations, SearchConfig};

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn var(name: &str) -> Term {
    Term::Var(name.into())
}

fn module(condition: Term) -> Module {
    Module {
        clauses: vec![Clause {
            name: "checked".into(),
            conclusion: app("accepted", vec![]),
            premises: vec![],
        }],
        clause_details: vec![ClauseDetail {
            clause: "checked".into(),
            premise_labels: vec![],
            bindings: vec![],
            conditions: vec![condition],
        }],
        ..Module::default()
    }
}

fn search(module: &Module, config: &SearchConfig) -> soda_relations::SearchResult {
    Relations::from_module(module)
        .unwrap()
        .search(&app("accepted", vec![]), config)
}

fn arithmetic_condition() -> Term {
    app(
        "eq",
        vec![app("add", vec![Term::Int(1), Term::Int(2)]), Term::Int(3)],
    )
}

fn verified_search(relations: &Relations) -> soda_relations::SearchResult {
    let result = relations
        .search_checked(&app("accepted", vec![]), &SearchConfig::default())
        .unwrap();
    support::assert_single_verified_answer(relations, &result);

    result
}

#[test]
fn true_and_false_conditions_filter_derivations_and_reject_forgeries() {
    let condition = arithmetic_condition();
    let good = module(condition);
    let relations = Relations::from_module(&good).unwrap();
    let result = verified_search(&relations);

    assert_eq!(result.attempts, 3);
    let bad = module(Term::Bool(false));
    let result = search(&bad, &SearchConfig::default());

    assert!(result.complete);
    assert!(result.answers.is_empty());
    let forged = Derivation {
        rule: "checked".into(),
        conclusion: app("accepted", vec![]),
        substitution: Substitution::new(),
        premises: vec![],
    };

    assert!(
        Relations::from_module(&bad)
            .unwrap()
            .verify(&forged)
            .is_err()
    );
}

#[test]
fn unknown_erroneous_and_budget_exhausted_conditions_are_incomplete() {
    for condition in [
        app("undefined", vec![]),
        Term::Int(1),
        app("div", vec![Term::Int(1), Term::Int(0)]),
    ] {
        let result = search(&module(condition), &SearchConfig::default());

        assert!(!result.complete);
        assert!(result.answers.is_empty());
    }

    let condition = arithmetic_condition();
    let result = search(
        &module(condition),
        &SearchConfig {
            max_attempts: 2,
            ..SearchConfig::default()
        },
    );

    assert!(!result.complete);
    assert!(result.answers.is_empty());
    assert_eq!(result.attempts, 2);
    let mut divergent = module(app("loop", vec![]));
    divergent.rules.push(Rule {
        name: "recur".into(),
        lhs: app("loop", vec![]),
        rhs: app("loop", vec![]),
        condition: None,
    });
    let result = search(
        &divergent,
        &SearchConfig {
            max_attempts: 4,
            ..SearchConfig::default()
        },
    );

    assert!(!result.complete);
    assert!(result.answers.is_empty());
    assert_eq!(result.attempts, 4);
}

#[test]
fn conditions_use_bindings_from_all_named_premises_and_semantic_rules() {
    let mut module = module(app("positive", vec![var("value")]));
    module.clauses[0]
        .premises
        .push(app("source", vec![var("value")]));
    module.clause_details[0]
        .premise_labels
        .push(Some("witness".into()));
    module.clauses.push(Clause {
        name: "source_fact".into(),
        conclusion: app("source", vec![Term::Int(2)]),
        premises: vec![],
    });
    module.rules.push(Rule {
        name: "is_positive".into(),
        lhs: app("positive", vec![var("n")]),
        rhs: app("gt", vec![var("n"), Term::Int(0)]),
        condition: None,
    });
    let relations = Relations::from_module(&module).unwrap();

    assert_eq!(
        relations.clause_detail("checked").unwrap().premise_labels,
        vec![Some("witness".into())]
    );
    let result = verified_search(&relations);

    assert_eq!(
        result.answers[0].derivation.substitution["value"],
        Term::Int(2)
    );
    let mut forged = result.answers[0].derivation.clone();
    forged.substitution.insert("value".into(), Term::Int(-2));

    assert!(relations.verify(&forged).is_err());
}

