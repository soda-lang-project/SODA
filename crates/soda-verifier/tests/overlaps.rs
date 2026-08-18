use soda_ast::{Rule, Term, match_term};
use soda_verifier::{CriticalPair, critical_pairs};

fn v(name: &str) -> Term {
    Term::Var(name.into())
}

fn a(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn rule(name: &str, lhs: Term, rhs: Term) -> Rule {
    Rule {
        name: name.into(),
        lhs,
        rhs,
        condition: None,
    }
}

fn only_pair_at(rules: &[Rule], position: &[usize]) -> CriticalPair {
    let mut pairs = critical_pairs(rules).unwrap().pairs;

    assert_eq!(pairs.len(), 1);
    let pair = pairs.pop().unwrap();

    assert_eq!(pair.position, position);

    pair
}

#[test]
fn root_overlaps_standardize_apart_before_occurs_check() {
    let rules = vec![
        rule("outer", a("f", vec![v("x")]), a("left", vec![v("x")])),
        rule(
            "inner",
            a("f", vec![a("g", vec![v("x")])]),
            a("right", vec![v("x")]),
        ),
    ];
    let report = critical_pairs(&rules).unwrap();

    assert_eq!(report.pairs.len(), 2);
    assert!(report.pairs.iter().all(|pair| pair.position.is_empty()));

    for pair in report.pairs {
        let outer = rules
            .iter()
            .find(|rule| rule.name == pair.outer_rule)
            .unwrap();
        let inner = rules
            .iter()
            .find(|rule| rule.name == pair.inner_rule)
            .unwrap();
        let outer_bindings = match_term(&outer.lhs, &pair.source).unwrap();
        let inner_bindings =
            match_term(&inner.lhs, pair.source.at(&pair.position).unwrap()).unwrap();

        assert_eq!(pair.outer_reduct, outer.rhs.substitute(&outer_bindings));
        assert_eq!(
            pair.inner_reduct,
            pair.source
                .replace(&pair.position, inner.rhs.substitute(&inner_bindings))
                .unwrap()
        );
    }
}

#[test]
fn includes_proper_self_overlaps_but_discards_trivial_root() {
    let rules = vec![rule(
        "nested",
        a("f", vec![a("f", vec![v("x")])]),
        a("g", vec![v("x")]),
    )];
    let pair = only_pair_at(&rules, &[0]);

    assert_eq!(pair.outer_rule, "nested");
    assert_eq!(pair.inner_rule, "nested");
    assert_ne!(pair.outer_reduct, pair.inner_reduct);
}

#[test]
fn repeated_variables_enforce_equality_during_overlap_unification() {
    let rules = vec![
        rule("same", a("f", vec![v("x"), v("x")]), Term::Bool(true)),
        rule(
            "different",
            a("f", vec![Term::Int(1), Term::Int(2)]),
            Term::Bool(false),
        ),
    ];

    assert!(critical_pairs(&rules).unwrap().pairs.is_empty());
}

#[test]
fn fresh_names_cannot_capture_adversarial_input_variable_names() {
    let rules = vec![
        rule(
            "first",
            a("f", vec![v("__soda_inner_0")]),
            a("g", vec![v("__soda_inner_0")]),
        ),
        rule(
            "second",
            a("f", vec![a("h", vec![v("__soda_outer_0")])]),
            v("__soda_outer_0"),
        ),
    ];
    let report = critical_pairs(&rules).unwrap();

    assert_eq!(report.pairs.len(), 2);

    for pair in report.pairs {
        let original = rules
            .iter()
            .find(|rule| rule.name == pair.outer_rule)
            .unwrap();
        let substitution = match_term(&original.lhs, &pair.source).unwrap();

        assert_eq!(original.rhs.substitute(&substitution), pair.outer_reduct);
    }
}

#[test]
fn excludes_guards_and_reports_interpretation_limits() {
    let rules = vec![
        rule("one", a("f", vec![v("x")]), Term::Int(1)),
        Rule {
            condition: Some(Term::Bool(true)),
            ..rule("two", a("f", vec![v("x")]), Term::Int(2))
        },
    ];
    let report = critical_pairs(&rules).unwrap();

    assert!(report.pairs.is_empty());
    assert_eq!(report.excluded_conditional_rules, vec!["two"]);
    assert!(
        report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("builtin"))
    );
    assert!(
        report
            .limitations
            .iter()
            .any(|limitation| limitation.contains("confluence"))
    );
}

#[test]
fn rejects_unsafe_and_duplicate_rules_before_analysis() {
    assert!(critical_pairs(&[rule("unsafe", a("f", vec![v("x")]), v("y"))]).is_err());
    assert!(critical_pairs(&[rule("variable", v("x"), Term::Int(0))]).is_err());
    let valid = rule("duplicate", a("f", vec![v("x")]), v("x"));

    assert!(critical_pairs(&[valid.clone(), valid]).is_err());
    let guarded = Rule {
        condition: Some(v("y")),
        ..rule("guard", a("f", vec![v("x")]), v("x"))
    };

    assert!(critical_pairs(&[guarded]).is_err());
}

#[test]
fn finds_nested_literal_overlap_and_skips_variable_positions() {
    let rules = vec![
        rule(
            "outer",
            a("f", vec![a("g", vec![Term::Int(0)]), v("x")]),
            v("x"),
        ),
        rule("zero", a("g", vec![Term::Int(0)]), Term::Int(1)),
    ];
    only_pair_at(&rules, &[0]);
}
