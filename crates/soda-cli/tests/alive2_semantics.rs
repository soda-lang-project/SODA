pub mod support;
use soda_ast::Term;
use soda_parser::{load_module_source, parse_term};
use soda_semantics::{Config, Engine, Status};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use support::evaluation::{evaluate, expect};

const SOURCE: &str = include_str!("../../../examples/alive2-integers.soda");

fn engine() -> Engine {
    let module = load_module_source("<alive2>", SOURCE, std::path::Path::new("."))
        .unwrap()
        .module;
    soda_verifier::validate_module(&module).unwrap();
    soda_sorts::validate_module(&module).unwrap();

    Engine::new(module.rules).unwrap()
}

fn numbers(ns: &[i64]) -> Term {
    ns.iter().rev().fold(Term::app("end", vec![]), |tail, n| {
        Term::app("cons", vec![Term::Int(*n), tail])
    })
}

fn value(width: i64, ns: &[i64]) -> Term {
    Term::app("values", vec![Term::Int(width), numbers(ns)])
}

fn number_set(term: &Term) -> BTreeSet<i64> {
    match term {
        Term::App(head, args) if head == "end" && args.is_empty() => BTreeSet::new(),
        Term::App(head, args) if head == "cons" && args.len() == 2 => {
            let Term::Int(n) = args[0] else {
                panic!("not a number: {term}")
            };

            let mut rest = number_set(&args[1]);
            assert!(rest.insert(n), "duplicate set element: {term}");

            rest
        }
        _ => panic!("not a number set: {term}"),
    }
}

fn outcomes(term: &Term) -> Vec<Term> {
    match term {
        Term::App(head, args) if head == "done" && args.is_empty() => vec![],
        Term::App(head, args) if head == "out" && args.len() == 2 => {
            let mut result = vec![args[0].clone()];
            result.extend(outcomes(&args[1]));

            result
        }
        _ => panic!("not an outcome list: {term}"),
    }
}

#[test]
fn integer_addition_matches_an_exhaustive_independent_small_width_oracle() {
    let engine = engine();

    for width in 1..=3 {
        let modulus = 1_i64 << width;
        let half = modulus / 2;
        let signed = |n| if n < half { n } else { n - modulus };

        for a in 0..modulus {
            for b in 0..modulus {
                let unsigned_overflow = a + b >= modulus;
                let sum = signed(a) + signed(b);
                let signed_overflow = !(-half..half).contains(&sum);

                for (flag, overflow) in [
                    ("wrapping", false),
                    ("nuw", unsigned_overflow),
                    ("nsw", signed_overflow),
                    ("nuw_nsw", unsigned_overflow || signed_overflow),
                ] {
                    let query = format!(
                        "instruction(sum({width}, {flag}, literal({a}), literal({b})), empty)"
                    );
                    let result = if overflow {
                        Term::app("poison", vec![Term::Int(width)])
                    } else {
                        value(width, &[(a + b) % modulus])
                    };

                    let expected = Term::app(
                        "out",
                        vec![
                            Term::app("returned", vec![result]),
                            Term::app("done", vec![]),
                        ],
                    );

                    assert_eq!(evaluate(&engine, &query), expected, "{query}");
                }

                let expected = if b == 0 {
                    "out(ub, done)".to_string()
                } else {
                    format!("out(returned(values({width}, cons({}, end))), done)", a / b)
                };

                expect(
                    &engine,
                    &format!("instruction(quotient({width}, literal({a}), literal({b})), empty)"),
                    &expected,
                );
            }
        }
    }

    expect(
        &engine,
        "instruction(sum(8, wrapping, literal(255), literal(1)), empty)",
        "out(returned(values(8, cons(0, end))), done)",
    );
}

#[test]
fn undef_is_a_set_and_overflow_and_zero_checks_range_over_all_choices() {
    let engine = engine();
    let result = evaluate(
        &engine,
        "add_value(2, wrapping, values(2, cons(0, cons(1, end))), values(2, cons(0, cons(1, end))))",
    );
    let Term::App(head, args) = result else {
        panic!("expected values")
    };

    assert_eq!(head, "values");
    assert_eq!(number_set(&args[1]), BTreeSet::from([0, 1, 2]));
    // One overflowing choice poisons the result, even though another is safe.
    expect(
        &engine,
        "add_value(2, nuw, values(2, cons(0, cons(3, end))), values(2, cons(1, end)))",
        "poison(2)",
    );
    expect(
        &engine,
        "divide_value(2, values(2, cons(3, end)), values(2, cons(1, cons(0, end))))",
        "ub",
    );
    expect(
        &engine,
        "instruction(quotient(2, literal(1), undef), empty)",
        "out(ub, done)",
    );
}

#[test]
fn poison_and_immediate_ub_remain_distinct_and_select_masks_only_poison() {
    let engine = engine();

    for (query, expected) in [
        (
            "instruction(sum(2, wrapping, toxic, literal(0)), empty)",
            "out(returned(poison(2)), done)",
        ),
        (
            "instruction(quotient(2, toxic, literal(1)), empty)",
            "out(returned(poison(2)), done)",
        ),
        (
            "instruction(quotient(2, toxic, literal(0)), empty)",
            "out(ub, done)",
        ),
        (
            "instruction(quotient(2, literal(1), toxic), empty)",
            "out(ub, done)",
        ),
        (
            "instruction(quotient(2, literal(3), literal(2)), empty)",
            "out(returned(values(2, cons(1, end))), done)",
        ),
        (
            "instruction(select_value(2, literal(1), literal(3), toxic), empty)",
            "out(returned(values(2, cons(3, end))), done)",
        ),
        (
            "instruction(select_value(2, literal(0), toxic, literal(2)), empty)",
            "out(returned(values(2, cons(2, end))), done)",
        ),
        (
            "instruction(select_value(2, toxic, literal(3), literal(2)), empty)",
            "out(returned(poison(2)), done)",
        ),
        // The division executes before select, so its UB cannot be masked.
        (
            "run_program(assign(quotient(2, literal(1), literal(0)), assign(select_value(2, literal(1), literal(3), reg(0)), ret(2, reg(0)))), empty)",
            "out(ub, done)",
        ),
    ] {
        expect(&engine, query, expected);
    }
}

