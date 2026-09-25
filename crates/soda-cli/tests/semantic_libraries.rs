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

