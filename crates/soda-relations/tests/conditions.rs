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

#[test]
fn freshness_conditions_check_bound_names_and_unknown_metavariables() {
    let x = Term::Name("x".into());
    let bound = Term::Binder {
        parameter: "x".into(),
        sort: "Expr".into(),
        body: Box::new(x.clone()),
    };

    for (body, expected) in [(x.clone(), 0), (bound, 1)] {
        let typed_binding = soda_sorts::requires_sort_check(&body);
        let mut definition = module(app("fresh", vec![x.clone(), body]));

        if typed_binding {
            definition.sorts.push(soda_ast::SortDecl {
                name: "Expr".into(),
                inductive: false,
            });
            definition.judgments.push(soda_ast::JudgmentDecl {
                name: "accepted".into(),
                arguments: vec![],
                kind: soda_ast::JudgmentKind::Relation,
            });
        }

        let result = search(&definition, &SearchConfig::default());

        assert!(result.complete);
        assert_eq!(result.answers.len(), expected);
    }

    let mut open = module(app("fresh", vec![x, var("body")]));
    open.clauses[0].conclusion = app("accepted", vec![var("body")]);
    let relations = Relations::from_module(&open).unwrap();
    let result = relations.search(
        &app("accepted", vec![var("unknown")]),
        &SearchConfig::default(),
    );

    assert!(!result.complete);
    assert!(result.answers.is_empty());
}

#[test]
fn malformed_clause_metadata_and_unsafe_conditions_are_rejected() {
    let base = module(Term::Bool(true));
    let mut unknown = base.clone();
    unknown.clause_details[0].clause = "missing".into();

    assert!(Relations::from_module(&unknown).is_err());
    let mut duplicate = base.clone();
    duplicate
        .clause_details
        .push(duplicate.clause_details[0].clone());

    assert!(Relations::from_module(&duplicate).is_err());
    let mut wrong_count = base.clone();
    wrong_count.clause_details[0].premise_labels.push(None);

    assert!(Relations::from_module(&wrong_count).is_err());
    let mut labels = base.clone();
    labels.clauses[0].premises = vec![app("a", vec![]), app("b", vec![])];
    labels.clause_details[0].premise_labels = vec![Some("same".into()), Some("same".into())];

    assert!(Relations::from_module(&labels).is_err());
    labels.clause_details[0].premise_labels = vec![Some("".into()), None];

    assert!(Relations::from_module(&labels).is_err());
    assert!(
        Relations::from_module(&module(app("eq", vec![var("unbound"), Term::Int(1)]))).is_err()
    );
}

#[test]
fn computed_bindings_feed_later_premises_and_reject_forged_results() {
    let mut model = module(Term::Bool(true));
    model.clauses[0].conclusion = app("accepted", vec![var("result")]);
    model.clauses[0].premises = vec![
        app("seed", vec![var("x")]),
        app("next", vec![var("mid"), var("result")]),
    ];
    model.clauses.extend([
        Clause {
            name: "seed".into(),
            conclusion: app("seed", vec![Term::Int(2)]),
            premises: vec![],
        },
        Clause {
            name: "next".into(),
            conclusion: app("next", vec![Term::Int(3), Term::Int(6)]),
            premises: vec![],
        },
    ]);
    model.clause_details[0].premise_labels = vec![None, None];
    model.clause_details[0].bindings = vec![soda_ast::ComputedBinding {
        before_premise: 1,
        pattern: var("mid"),
        value: app("add", vec![var("x"), Term::Int(1)]),
    }];
    let relations = Relations::from_module(&model).unwrap();
    let result = relations.search(
        &app("accepted", vec![var("answer")]),
        &SearchConfig::default(),
    );
    assert!(result.complete);
    assert_eq!(result.answers.len(), 1);
    assert_eq!(result.answers[0].term, app("accepted", vec![Term::Int(6)]));
    relations.verify(&result.answers[0].derivation).unwrap();
    let mut forged = result.answers[0].derivation.clone();
    forged.substitution.insert("mid".into(), Term::Int(4));
    assert!(relations.verify(&forged).is_err());
    let mut forged = result.answers[0].derivation.clone();
    forged.substitution.insert("x".into(), Term::Int(99));
    forged.premises[0].conclusion = app("seed", vec![Term::Int(99)]);
    assert!(relations.verify(&forged).is_err());
}

#[test]
fn computed_binding_failures_are_incomplete_and_mismatch_is_exhaustive() {
    let mut model = module(Term::Bool(true));
    model.clauses[0].conclusion = app("accepted", vec![var("out")]);
    model.clause_details[0].bindings = vec![soda_ast::ComputedBinding {
        before_premise: 0,
        pattern: var("out"),
        value: app("add", vec![Term::Int(2), Term::Int(3)]),
    }];
    let relations = Relations::from_module(&model).unwrap();
    let result = relations.search(
        &app("accepted", vec![Term::Int(4)]),
        &SearchConfig::default(),
    );
    assert!(result.complete);
    assert!(result.answers.is_empty());
    let limited = relations.search(
        &app("accepted", vec![var("q")]),
        &SearchConfig {
            max_attempts: 1,
            ..SearchConfig::default()
        },
    );
    assert!(!limited.complete);
    assert!(limited.answers.is_empty());
    model.clause_details[0].bindings[0].value = app("add", vec![var("input"), Term::Int(3)]);
    let relations = Relations::from_module(&model).unwrap();
    let open = relations.search(&app("accepted", vec![var("q")]), &SearchConfig::default());
    assert!(!open.complete);
    assert!(open.answers.is_empty());
    model.clause_details[0].bindings[0].value = app("div", vec![Term::Int(1), Term::Int(0)]);
    let relations = Relations::from_module(&model).unwrap();
    let stuck = relations.search(&app("accepted", vec![var("q")]), &SearchConfig::default());
    assert!(!stuck.complete);
    assert!(stuck.answers.is_empty());
}

#[test]
fn checker_recomputes_a_trailing_binding_without_trusting_substitution() {
    let mut model = module(Term::Bool(true));
    model.clauses[0].conclusion = app("accepted", vec![var("out")]);
    model.clause_details[0].bindings = vec![soda_ast::ComputedBinding {
        before_premise: 0,
        pattern: var("out"),
        value: app("add", vec![Term::Int(2), Term::Int(3)]),
    }];
    let relations = Relations::from_module(&model).unwrap();
    let result = relations.search(&app("accepted", vec![var("q")]), &SearchConfig::default());
    relations.verify(&result.answers[0].derivation).unwrap();
    let mut forged = result.answers[0].derivation.clone();
    forged.conclusion = app("accepted", vec![Term::Int(6)]);
    forged.substitution.insert("out".into(), Term::Int(6));
    let error = relations.verify(&forged).unwrap_err();
    assert!(error.contains("computed binding"), "{error}");
}
