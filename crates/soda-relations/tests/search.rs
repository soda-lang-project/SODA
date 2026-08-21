use soda_ast::{Clause, Substitution, Term};
use soda_relations::{Derivation, Relations, SearchConfig, answer_matches};

fn v(name: &str) -> Term {
    Term::Var(name.into())
}

fn a(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn clause(name: &str, conclusion: Term, premises: Vec<Term>) -> Clause {
    Clause {
        name: name.into(),
        conclusion,
        premises,
    }
}

fn fact(name: &str, predicate: &str, value: i64) -> Clause {
    clause(name, a(predicate, vec![Term::Int(value)]), vec![])
}

#[test]
fn skipped_heads_keep_attempt_budgets_and_fresh_names() {
    let relations = Relations::new(vec![
        clause("other", a("other", vec![v("x")]), vec![]),
        clause("answer", a("answer", vec![v("x")]), vec![]),
    ])
    .unwrap();

    let query = a("answer", vec![v("_soda_internal_0")]);
    let result = relations.search(&query, &SearchConfig::default());

    assert!(result.complete);
    assert_eq!(result.attempts, 2);
    assert_eq!(
        result.answers[0].derivation.substitution["x"],
        v("_soda_internal_0")
    );
    relations.verify(&result.answers[0].derivation).unwrap();
    let limited = relations.search(
        &query,
        &SearchConfig {
            max_attempts: 1,
            ..SearchConfig::default()
        },
    );

    assert!(!limited.complete);
    assert_eq!(limited.attempts, 1);
    assert!(limited.answers.is_empty());
}

#[test]
fn shared_variables_constrain_later_premises_and_backtracking() {
    let relations = Relations::new(vec![
        clause(
            "both",
            a("both", vec![v("x")]),
            vec![a("p", vec![v("x")]), a("q", vec![v("x")])],
        ),
        fact("p1", "p", 1),
        fact("p2", "p", 2),
        fact("q2", "q", 2),
    ])
    .unwrap();
    let query = a("both", vec![v("answer")]);
    let result = relations.search(&query, &SearchConfig::default());

    assert!(result.complete);
    assert_eq!(result.answers.len(), 1);
    assert_eq!(result.answers[0].bindings["answer"], Term::Int(2));
    assert!(answer_matches(&query, &result.answers[0]));
    relations.verify(&result.answers[0].derivation).unwrap();
}

#[test]
fn premise_only_existential_variables_are_shared_and_instantiated() {
    let relations = Relations::new(vec![
        clause(
            "exists",
            a("exists", vec![]),
            vec![a("p", vec![v("witness")]), a("q", vec![v("witness")])],
        ),
        fact("p3", "p", 3),
        fact("q3", "q", 3),
    ])
    .unwrap();
    let result = relations.search(&a("exists", vec![]), &SearchConfig::default());

    assert!(result.complete);
    assert_eq!(result.answers.len(), 1);
    assert_eq!(
        result.answers[0].derivation.substitution["witness"],
        Term::Int(3)
    );
    relations.verify(&result.answers[0].derivation).unwrap();
}

#[test]
fn recursive_calls_and_user_variable_names_cannot_capture_fresh_variables() {
    let relations = Relations::new(vec![
        clause("base", a("nat", vec![a("zero", vec![])]), vec![]),
        clause(
            "successor",
            a("nat", vec![a("s", vec![v("_soda_internal_0")])]),
            vec![a("nat", vec![v("_soda_internal_0")])],
        ),
    ])
    .unwrap();
    let query = a(
        "nat",
        vec![a("s", vec![a("s", vec![v("_soda_internal_1")])])],
    );
    let result = relations.search(
        &query,
        &SearchConfig {
            max_depth: 3,
            ..SearchConfig::default()
        },
    );

    assert!(!result.answers.is_empty());
    for answer in &result.answers {
        assert!(answer_matches(&query, answer));
        relations.verify(&answer.derivation).unwrap();
    }

    assert!(!result.complete);
}

#[test]
fn empty_and_finitely_failed_search_are_complete() {
    let empty = Relations::new(vec![]).unwrap();
    let result = empty.search(&a("p", vec![]), &SearchConfig::default());

    assert!(result.complete);
    assert!(result.answers.is_empty());
    assert_eq!(result.attempts, 0);
    let relations = Relations::new(vec![fact("p", "p", 1)]).unwrap();
    let result = relations.search(&a("q", vec![Term::Int(1)]), &SearchConfig::default());

    assert!(result.complete);
    assert!(result.answers.is_empty());
}

#[test]
fn cycles_and_zero_budgets_are_incomplete_without_false_answers() {
    let relations = Relations::new(vec![clause(
        "loop",
        a("p", vec![v("x")]),
        vec![a("p", vec![v("x")])],
    )])
    .unwrap();

    for config in [
        SearchConfig {
            max_depth: 3,
            ..SearchConfig::default()
        },
        SearchConfig {
            max_attempts: 0,
            ..SearchConfig::default()
        },
        SearchConfig {
            max_answers: 0,
            ..SearchConfig::default()
        },
    ] {
        let result = relations.search(&a("p", vec![Term::Int(1)]), &config);

        assert!(!result.complete);
        assert!(result.answers.is_empty());
    }
}

#[test]
fn answer_cap_returns_checked_answers_and_reports_incompleteness() {
    let relations = Relations::new(vec![fact("p1", "p", 1), fact("p2", "p", 2)]).unwrap();
    let result = relations.search(
        &a("p", vec![v("x")]),
        &SearchConfig {
            max_answers: 1,
            ..SearchConfig::default()
        },
    );

    assert_eq!(result.answers.len(), 1);
    assert!(!result.complete);
    relations.verify(&result.answers[0].derivation).unwrap();
}

#[test]
fn repeated_query_variables_are_consistency_constraints() {
    let relations = Relations::new(vec![clause(
        "unequal",
        a("pair", vec![Term::Int(1), Term::Int(2)]),
        vec![],
    )])
    .unwrap();
    let result = relations.search(&a("pair", vec![v("x"), v("x")]), &SearchConfig::default());

    assert!(result.complete);
    assert!(result.answers.is_empty());
}

