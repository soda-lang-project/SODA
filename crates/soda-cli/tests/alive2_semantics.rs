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

#[test]
fn freeze_is_a_stable_ssa_choice_rather_than_an_undef_value() {
    let engine = engine();

    for operand in ["undef", "toxic"] {
        let query = format!(
            "run_program(assign(freeze(2, {operand}), assign(sum(2, wrapping, reg(0), reg(0)), ret(2, reg(0)))), empty)"
        );
        let result = outcomes(&evaluate(&engine, &query));

        assert_eq!(result.len(), 4);
        let expected: BTreeSet<_> = [0, 2]
            .map(|n| Term::app("returned", vec![value(2, &[n])]))
            .into();

        assert_eq!(result.into_iter().collect::<BTreeSet<_>>(), expected);
    }
    // Repeated uses of a non-frozen undef register yield every i2 pattern.
    let query = "run_program(assign(sum(2, wrapping, reg(0), reg(0)), ret(2, reg(0))), push(values(2, cons(0, cons(1, cons(2, cons(3, end))))), empty))";
    let result = outcomes(&evaluate(&engine, query));

    assert_eq!(result.len(), 1);
    let Term::App(_, returned) = &result[0] else {
        panic!("expected returned")
    };

    let Term::App(_, args) = &returned[0] else {
        panic!("expected values")
    };

    assert_eq!(number_set(&args[1]), BTreeSet::from([0, 1, 2, 3]));
    expect(
        &engine,
        "instruction(freeze(2, literal(3)), empty)",
        "out(returned(values(2, cons(3, end))), done)",
    );
    // Separate freezes make independent choices; reusing one freeze does not.

    let independent = evaluate(
        &engine,
        "run_program(assign(freeze(1, undef), assign(freeze(1, undef), assign(sum(1, wrapping, reg(0), reg(1)), ret(1, reg(0))))), empty)",
    );
    let independent = outcomes(&independent);

    assert_eq!(independent.len(), 4);
    assert_eq!(
        independent.into_iter().collect::<BTreeSet<_>>(),
        [0, 1]
            .map(|n| Term::app("returned", vec![value(1, &[n])]))
            .into()
    );
}

#[test]
fn refinement_is_directional_and_covers_every_target_execution() {
    let engine = engine();

    for (query, expected) in [
        ("value_refines(poison(2), values(2, cons(1, end)))", "true"),
        ("value_refines(values(2, cons(1, end)), poison(2))", "false"),
        (
            "value_refines(values(2, cons(1, cons(2, end))), values(2, cons(1, end)))",
            "true",
        ),
        (
            "value_refines(values(2, cons(1, end)), values(2, cons(1, cons(2, end))))",
            "false",
        ),
        ("outcome_refines(ub, returned(poison(2)))", "true"),
        ("outcome_refines(returned(poison(2)), ub)", "false"),
        (
            "refines(out(returned(values(2, cons(1, end))), done), out(returned(values(2, cons(1, end))), out(returned(values(2, cons(2, end))), done)))",
            "false",
        ),
        (
            "refines(instruction(sum(2, nuw, literal(3), literal(1)), empty), instruction(sum(2, wrapping, literal(3), literal(1)), empty))",
            "true",
        ),
        (
            "refines(instruction(sum(2, wrapping, literal(3), literal(1)), empty), instruction(sum(2, nuw, literal(3), literal(1)), empty))",
            "false",
        ),
    ] {
        expect(&engine, query, expected);
    }

    let undef = "run_program(ret(2, undef), empty)";
    let frozen = "run_program(assign(freeze(2, undef), ret(2, reg(0))), empty)";
    expect(&engine, &format!("refines({undef}, {frozen})"), "true");
    expect(&engine, &format!("refines({frozen}, {undef})"), "false");
}

#[test]
fn malformed_widths_and_registers_do_not_turn_into_llvm_ub() {
    let engine = engine();

    for query in [
        "run_program(ret(0, literal(0)), empty)",
        "run_program(ret(9, literal(0)), empty)",
        "run_program(ret(2, literal(4)), empty)",
        "run_program(ret(2, literal(-1)), empty)",
        "run_program(ret(2, reg(0)), empty)",
        "run_program(ret(2, reg(0)), push(values(1, cons(0, end)), empty))",
        "run_program(ret(2, reg(0)), push(values(2, end), empty))",
        "run_program(ret(2, reg(0)), push(values(2, cons(4, end)), empty))",
    ] {
        let result = engine.evaluate(&parse_term(query).unwrap(), &Config::default());

        assert!(
            matches!(result.status, Status::Stuck(_)),
            "{query}: {result:?}"
        );
    }

    let query = parse_term("instruction(sum(2, wrapping, literal(3), literal(1)), empty)").unwrap();
    let exhausted = engine.evaluate(
        &query,
        &Config {
            fuel: 1,
            ..Config::default()
        },
    );

    assert!(matches!(exhausted.status, Status::Limit(_)));
}

#[test]
fn shipped_demonstrations_execute_in_both_interpreters_and_cli() {
    let module = load_module_source("<alive2>", SOURCE, std::path::Path::new("."))
        .unwrap()
        .module;
    let engine = engine();

    for query in &module.evaluations {
        let operational = engine.evaluate(query, &Config::default());
        let compositional = engine.denote(query, &Config::default());

        assert_eq!(
            operational.status,
            Status::NormalForm,
            "{query}: {operational:?}"
        );
        assert_eq!(
            compositional.status,
            Status::NormalForm,
            "{query}: {compositional:?}"
        );
        assert_eq!(operational.term, compositional.term, "{query}");
    }

    let output = Command::new(env!("CARGO_BIN_EXE_soda"))
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["compare", "examples/alive2-integers.soda"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
