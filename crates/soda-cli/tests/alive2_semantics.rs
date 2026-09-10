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

