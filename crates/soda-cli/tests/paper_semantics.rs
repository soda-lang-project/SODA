use soda_parser::{parse_module, parse_term};
use soda_relations::{Relations, SearchConfig, SearchResult};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

const STLC: &str = include_str!("../../../examples/stlc.soda");
const CONTEXTS: &str = include_str!("../../../examples/evaluation-contexts.soda");
const CCS: &str = include_str!("../../../examples/ccs.soda");

fn relations(source: &str) -> Relations {
    Relations::from_module(&parse_module(source).unwrap()).unwrap()
}

fn search(relations: &Relations, source: &str) -> SearchResult {
    let query = parse_term(source).unwrap();
    let result = relations
        .search_checked(&query, &SearchConfig::default())
        .unwrap();
    assert!(result.complete, "incomplete search for {source}");
    for answer in &result.answers {
        assert!(soda_relations::answer_matches(&query, answer));
        relations.verify(&answer.derivation).unwrap();
    }

    result
}

fn answers(relations: &Relations, query: &str, expected: &[&str]) {
    let result = search(relations, query);
    let actual: BTreeSet<_> = result.answers.iter().map(|a| a.term.clone()).collect();
    let expected: BTreeSet<_> = expected.iter().map(|s| parse_term(s).unwrap()).collect();
    assert_eq!(actual, expected, "{query}");
    assert_eq!(
        result.answers.len(),
        expected.len(),
        "duplicate answers: {query}"
    );
}

#[test]
fn stlc_types_closures_shadowing_and_call_by_value() {
    let r = relations(STLC);
    answers(
        &r,
        "typing(empty, lam(bool_type, var(zero)), ?t)",
        &["typing(empty, lam(bool_type, var(zero)), arrow(bool_type, bool_type))"],
    );
    for body in ["var(succ(zero))", "var(zero)"] {
        let query =
            format!("evaluates(nil, app(app(lam(bool_type, lam(bool_type, {body})), tt), ff), ?v)");
        let value = if body == "var(zero)" { "no" } else { "yes" };

        answers(&r, &query, &[&query.replace("?v", value)]);
    }

    let query = "evaluates(nil, app(lam(arrow(bool_type, bool_type), app(var(zero), tt)), lam(bool_type, var(zero))), ?v)";
    answers(&r, query, &[&query.replace("?v", "yes")]);
    for query in [
        "typing(empty, app(tt, ff), ?t)",
        "typing(empty, var(zero), ?t)",
        "typing(empty, cond(tt, tt, lam(bool_type, var(zero))), ?t)",
        "typing(empty, app(lam(bool_type, var(zero)), lam(bool_type, var(zero))), ?t)",
        "evaluates(nil, app(lam(bool_type, tt), var(zero)), ?v)",
    ] {
        answers(&r, query, &[]);
    }

    answers(
        &r,
        "evaluates(nil, cond(tt, tt, var(zero)), ?v)",
        &["evaluates(nil, cond(tt, tt, var(zero)), yes)"],
    );
}

#[test]
fn evaluation_contexts_enforce_order_and_do_not_reduce_under_values() {
    let r = relations(CONTEXTS);

    for (input, output) in [
        (
            "pair(cond(tt, ff, tt), cond(ff, ff, tt))",
            "pair(ff, cond(ff, ff, tt))",
        ),
        ("pair(ff, cond(ff, ff, tt))", "pair(ff, tt)"),
        ("first(pair(tt, cond(ff, ff, tt)))", "first(pair(tt, tt))"),
        ("first(pair(tt, tt))", "tt"),
        ("cond(tt, ff, cond(tt, tt, ff))", "ff"),
    ] {
        answers(
            &r,
            &format!("step({input}, ?next)"),
            &[&format!("step({input}, {output})")],
        );
    }

    for query in [
        "step(pair(tt, ff), ?next)",
        "step(first(tt), ?next)",
        "step(pair(first(tt), cond(tt, ff, tt)), ?next)",
    ] {
        answers(&r, query, &[]);
    }

    let mut term = parse_term("first(pair(cond(ff, ff, tt), cond(tt, ff, tt)))").unwrap();

    for _ in 0..3 {
        let result = search(&r, &format!("step({term}, ?next)"));

        assert_eq!(result.answers.len(), 1);
        term = result.answers[0].bindings["next"].clone();
    }

    assert_eq!(term, parse_term("tt").unwrap());
    answers(&r, &format!("step({term}, ?next)"), &[]);
}

#[test]
fn ccs_enumerates_choice_interleavings_synchronization_and_restriction() {
    let r = relations(CCS);
    let p = "parallel(prefix(input(a), stop), prefix(output(a), stop))";
    answers(
        &r,
        &format!("transition({p}, ?label, ?next)"),
        &[
            &format!("transition({p}, input(a), parallel(stop, prefix(output(a), stop)))"),
            &format!("transition({p}, output(a), parallel(prefix(input(a), stop), stop))"),
            &format!("transition({p}, tau, parallel(stop, stop))"),
        ],
    );
    answers(
        &r,
        &format!("transition(restrict(a, {p}), ?label, ?next)"),
        &[&format!(
            "transition(restrict(a, {p}), tau, restrict(a, parallel(stop, stop)))"
        )],
    );
    answers(
        &r,
        "transition(choice(prefix(input(a), stop), prefix(output(b), stop)), ?label, ?next)",
        &[
            "transition(choice(prefix(input(a), stop), prefix(output(b), stop)), input(a), stop)",
            "transition(choice(prefix(input(a), stop), prefix(output(b), stop)), output(b), stop)",
        ],
    );
    answers(
        &r,
        "transition(restrict(a, prefix(input(b), stop)), ?label, ?next)",
        &["transition(restrict(a, prefix(input(b), stop)), input(b), restrict(a, stop))"],
    );

    for query in [
        "transition(parallel(prefix(input(a), stop), prefix(output(b), stop)), tau, ?next)",
        "transition(parallel(prefix(input(a), stop), prefix(input(a), stop)), tau, ?next)",
        "transition(restrict(a, prefix(input(a), stop)), ?label, ?next)",
        "transition(stop, ?label, ?next)",
    ] {
        answers(&r, query, &[]);
    }
}

