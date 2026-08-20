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

