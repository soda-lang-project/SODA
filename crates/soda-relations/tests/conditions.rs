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

