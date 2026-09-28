pub mod support;
use soda_parser::{load_module_source, parse_term};
use soda_semantics::{Config, Engine, Status};

const STATE: &str = include_str!("../../soda-parser/stdlib/state.soda");
const EXCEPTIONS: &str = include_str!("../../soda-parser/stdlib/exceptions.soda");
const PROBABILITY: &str = include_str!("../../soda-parser/stdlib/probability.soda");
const COLLECTIONS: &str = include_str!("../../soda-parser/stdlib/collections.soda");

fn engine(source: &str) -> Engine {
    let module = load_module_source("<library>", source, std::path::Path::new("."))
        .unwrap()
        .module;
    soda_sorts::validate_module(&module).unwrap();
    soda_verifier::validate_module(&module).unwrap();
    Engine::new(module.rules).unwrap()
}

fn value(engine: &Engine, input: &str) -> soda_ast::Term {
    let result = engine.evaluate(&parse_term(input).unwrap(), &Config::default());
    assert_eq!(result.status, Status::NormalForm, "{input}: {result:?}");
    result.term
}

fn equal(engine: &Engine, left: &str, right: &str) {
    assert_eq!(
        value(engine, left),
        value(engine, right),
        "{left} != {right}"
    );
}

#[test]
fn state_monad_laws_and_state_threading() {
    let engine = engine(STATE);
    for initial in [-2, 0, 7] {
        for n in [-1, 0, 4] {
            equal(
                &engine,
                &format!(
                    "apply(state_bind(state_unit({n}), bind(x: Int, state_put(@x))), {initial})"
                ),
                &format!("apply(state_put({n}), {initial})"),
            );
            equal(
                &engine,
                &format!("apply(state_bind(state_get, bind(x: Int, state_unit(@x))), {initial})"),
                &format!("apply(state_get, {initial})"),
            );
            equal(
                &engine,
                &format!(
                    "apply(state_bind(state_bind(state_get, bind(x: Int, state_put(add(@x, {n})))), bind(y: Int, state_get)), {initial})"
                ),
                &format!(
                    "apply(state_bind(state_get, bind(x: Int, state_bind(state_put(add(@x, {n})), bind(y: Int, state_get)))), {initial})"
                ),
            );
            equal(
                &engine,
                &format!(
                    "apply(state_bind(state_put({n}), bind(ignored: Int, state_get)), {initial})"
                ),
                &format!("state_answer({n}, {n})"),
            );
        }
    }
}

#[test]
fn exceptions_obey_monad_laws_and_propagate_before_division() {
    let engine = engine(EXCEPTIONS);
    for n in [-3, 0, 9] {
        equal(
            &engine,
            &format!(
                "exception_bind(exception_unit({n}), bind(x: Int, exception_unit(add(@x, 1))))"
            ),
            &format!("exception_unit(add({n}, 1))"),
        );
    }
    for computation in ["exception_returned(7)", "exception_raised(exception_error)"] {
        equal(
            &engine,
            &format!("exception_bind({computation}, bind(x: Int, exception_unit(@x)))"),
            computation,
        );
        equal(
            &engine,
            &format!(
                "exception_bind(exception_bind({computation}, bind(x: Int, exception_div(@x, 2))), bind(y: Int, exception_unit(add(@y, 1))))"
            ),
            &format!(
                "exception_bind({computation}, bind(x: Int, exception_bind(exception_div(@x, 2), bind(y: Int, exception_unit(add(@y, 1))))))"
            ),
        );
    }
    equal(
        &engine,
        "exception_bind(exception_raised(exception_error), bind(x: Int, exception_div(@x, 0)))",
        "exception_raised(exception_error)",
    );
    equal(
        &engine,
        "exception_div(1, 0)",
        "exception_raised(exception_zero_divisor)",
    );
    equal(
        &engine,
        "exception_catch(exception_div(1, 0), bind(e: ExceptionError, exception_unit(42)))",
        "exception_returned(42)",
    );
}

#[test]
fn probability_laws_compare_exact_weights_extensionally() {
    let engine = engine(PROBABILITY);
    let same = |left: &str, right: &str| {
        equal(&engine, &format!("prob_equal({left}, {right})"), "true");
    };
    for p in ["fraction(0, 1)", "fraction(1, 3)", "fraction(1, 1)"] {
        let distribution = format!("prob_mix({p}, prob_unit(true), prob_zero)");
        same(
            &format!("prob_bind({distribution}, bind(x: Bool, prob_unit(@x)))"),
            &distribution,
        );
        same(
            &format!(
                "prob_bind(prob_bind({distribution}, bind(x: Bool, prob_unit(not(@x)))), bind(y: Bool, prob_unit(not(@y))))"
            ),
            &format!(
                "prob_bind({distribution}, bind(x: Bool, prob_bind(prob_unit(not(@x)), bind(y: Bool, prob_unit(not(@y))))))"
            ),
        );
    }
    same(
        "prob_bind(prob_unit(true), bind(x: Bool, prob_unit(not(@x))))",
        "prob_unit(false)",
    );
    same(
        "prob_mix(fraction(1, 2), prob_unit(true), prob_zero)",
        "prob_weights(fraction(0, 1), fraction(1, 2))",
    );
    equal(
        &engine,
        "rle(prob_mass(prob_mix(fraction(1, 3), prob_unit(true), prob_zero)), fraction(1, 3))",
        "true",
    );
    equal(&engine, "prob_le(prob_zero, prob_unit(false))", "true");
}

#[test]
fn collections_preserve_identity_associativity_and_fold_values() {
    let engine = engine(COLLECTIONS);
    let xs = "cons(1, cons(2, end))";
    let ys = "cons(3, end)";
    equal(&engine, &format!("list_append(end, {xs})"), xs);
    equal(&engine, &format!("list_append({xs}, end)"), xs);
    equal(
        &engine,
        &format!("list_append(list_append({xs}, {ys}), {xs})"),
        &format!("list_append({xs}, list_append({ys}, {xs}))"),
    );
    equal(&engine, &format!("list_map(bind(x: Int, @x), {xs})"), xs);
    equal(
        &engine,
        &format!("list_length(list_append({xs}, {ys}))"),
        "3",
    );
    equal(
        &engine,
        &format!("list_fold(bind(x: Int, bind(y: Int, add(@x, @y))), 0, list_append({xs}, {ys}))"),
        "6",
    );
    equal(&engine, &format!("list_member(2, {xs})"), "true");
    equal(&engine, &format!("list_member(9, {xs})"), "false");
}

#[test]
fn imported_libraries_execute_through_both_cli_evaluators() {
    for command in ["run", "denote"] {
        let output = support::run(&[command, "examples/semantic-libraries.soda"]);
        assert!(
            output.status.success(),
            "{command}: {}\n{}",
            support::stdout(&output),
            support::stderr(&output)
        );
        let stdout = support::stdout(&output);
        for expected in [
            "state_answer(14, 5)",
            "exception_returned(5)",
            "exception_returned(42)",
            "cons(2, cons(4, end))",
        ] {
            assert!(
                stdout.contains(expected),
                "{command} missing {expected}: {stdout}"
            );
        }
    }
}

#[test]
fn imported_surface_syntax_lowers_to_checked_equations_in_both_engines() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/semantic-libraries.soda");
    let loaded = soda_parser::load_module(&path).unwrap();
    soda_sorts::validate_module(&loaded.module).unwrap();
    soda_verifier::validate_module(&loaded.module).unwrap();
    let engine = Engine::new(loaded.module.rules).unwrap();
    let expected = [
        "state_answer(14, 5)",
        "exception_returned(5)",
        "exception_returned(42)",
        "true",
        "cons(2, cons(4, end))",
        "6",
    ];
    assert_eq!(loaded.module.evaluations.len(), expected.len());
    for (query, expected) in loaded.module.evaluations.iter().zip(expected) {
        for result in [
            engine.evaluate(query, &Config::default()),
            engine.denote(query, &Config::default()),
        ] {
            assert_eq!(result.status, Status::NormalForm, "{query}: {result:?}");
            assert_eq!(result.term, parse_term(expected).unwrap(), "{query}");
        }
    }
}

#[test]
fn prelude_reuses_checked_natural_theorems_and_executes_all_library_families() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/standard-library.soda");
    let module = soda_parser::load_module(&path).unwrap().module;
    soda_sorts::validate_module(&module).unwrap();
    soda_verifier::validate_module(&module).unwrap();
    let engine = Engine::new(module.rules).unwrap();
    let expected = [
        "6",
        "3",
        "true",
        "state_answer(14, 5)",
        "exception_returned(43)",
        "true",
    ];
    assert_eq!(module.evaluations.len(), expected.len());
    for (term, expected) in module.evaluations.iter().zip(expected) {
        for result in [
            engine.evaluate(term, &Config::default()),
            engine.denote(term, &Config::default()),
        ] {
            assert_eq!(result.status, Status::NormalForm, "{term}: {result:?}");
            assert_eq!(result.term, parse_term(expected).unwrap());
        }
    }
    for command in ["check", "prove", "compare"] {
        let output = support::run(&[command, "examples/standard-library.soda"]);
        assert!(
            output.status.success(),
            "{command}: {}{}",
            support::stdout(&output),
            support::stderr(&output)
        );
    }
}

#[test]
fn natural_arithmetic_agrees_with_integer_arithmetic() {
    let engine = engine("module Naturals; import soda.data.nat;");
    let nat = |n: usize| format!("{}zero{}", "succ(".repeat(n), ")".repeat(n));
    for a in 0..=4 {
        for b in 0..=4 {
            equal(
                &engine,
                &format!("nat_to_int(nat_add({}, {}))", nat(a), nat(b)),
                &(a + b).to_string(),
            );
            equal(
                &engine,
                &format!("nat_to_int(nat_mul({}, {}))", nat(a), nat(b)),
                &(a * b).to_string(),
            );
        }
    }
}

#[test]
fn typed_judgments_reuse_imported_syntax_notation_and_checked_derivations() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/typed-judgments.soda");
    let loaded = soda_parser::load_module(path).unwrap();
    soda_sorts::validate_module(&loaded.module).unwrap();
    soda_verifier::verify_module(&loaded.module).unwrap();
    let relations = soda_relations::Relations::from_module(&loaded.module).unwrap();
    for (query, expected) in loaded.module.queries.iter().zip([
        "typing(empty, plus(lit(1), lit(2)), int_type)",
        "typing(empty, lit(3), int_type)",
        "small_step(plus(lit(0), lit(4)), lit(4))",
        "big_step(lit(5), 5)",
        "meaning(lit(6), empty, 6)",
        "triple(lit(7), lit(7), lit(7))",
        "satisfies(empty, lit(8))",
        "meaning(lit(9), empty, 9)",
        "meaning(lit(10), empty, 10)",
        "big_step(plus(lit(2), plus(lit(3), lit(4))), 9)",
        "meaning(plus(lit(2), lit(3)), empty, 5)",
    ]) {
        let search = relations
            .search_checked(query, &soda_relations::SearchConfig::default())
            .unwrap();
        assert!(search.complete, "{query}");
        assert_eq!(search.answers.len(), 1, "{query}");
        assert_eq!(search.answers[0].term, parse_term(expected).unwrap());
        relations.verify(&search.answers[0].derivation).unwrap();
    }
    assert_eq!(loaded.module.queries.len(), 11);
}

