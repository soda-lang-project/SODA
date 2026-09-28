use soda_parser::{load_module, load_module_source, parse_term, parse_term_in};
use soda_semantics::{Config, Engine, Status};
use std::path::PathBuf;
pub mod support;

fn assert_both(module: &soda_ast::Module, expression: &str, expected: &str) {
    let engine = Engine::new(module.rules.clone()).unwrap();
    let input = parse_term_in(module, expression).unwrap();
    let expected = parse_term(expected).unwrap();
    for result in [
        engine.evaluate(&input, &Config::default()),
        engine.denote(&input, &Config::default()),
    ] {
        assert_eq!(
            result.status,
            Status::NormalForm,
            "{expression}: {result:?}"
        );
        assert_eq!(result.term, expected, "{expression}");
    }
}

#[test]
fn combined_records_lists_arrows_and_do_have_concrete_results() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/surface-syntax.soda");
    let loaded = load_module(&path).unwrap();
    let module = &loaded.module;
    soda_sorts::validate_module(module).unwrap();
    soda_verifier::validate_module(module).unwrap();
    let expected = [
        "observation(cons(2,cons(3,cons(4,end))),state_answer(15,15))",
        "8",
        "exception_raised(exception_zero_divisor)",
    ];
    assert_eq!(module.evaluations.len(), expected.len());
    for (input, expected) in module.evaluations.iter().zip(expected) {
        assert_both(module, &input.to_string(), expected);
    }
    assert_both(
        module,
        "apply(do State { x <- state_get; state_tick; return x; }, 6)",
        "state_answer(6,7)",
    );
    assert_both(
        module,
        "first_sample(observation {outcome:state_answer(0,0),samples:[9,8]})",
        "9",
    );
}

