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

