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

#[test]
fn do_binders_preserve_shadowing_and_capture_avoidance_under_rewriting() {
    let source = "module Hygiene; import soda.effects.state;
    function preserve(Int -> Int): Int -> StateResult;
    rule preserve_value: preserve(?f) => do State { x <- state_unit(7); return apply(?f,x); };
    function meta(Int): Int -> StateResult;
    rule distinct_meta: meta(?x) => do State { x <- state_get; return add(x,?x); };
    ";
    let loaded =
        load_module_source("Hygiene", source, PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();
    soda_sorts::validate_module(&loaded.module).unwrap();
    assert_both(
        &loaded.module,
        "apply(apply(bind(x:Int,preserve(bind(y:Int,add(@x,@y)))),5),0)",
        "state_answer(12,0)",
    );
    assert_both(&loaded.module, "apply(meta(8),3)", "state_answer(11,3)");
    assert_both(
        &loaded.module,
        "apply(bind(x:Int, apply(do State { x <- state_get; return x; }, @x)),7)",
        "state_answer(7,7)",
    );
    assert_both(
        &loaded.module,
        "apply(do State { x <- state_unit(2); return apply(bind(x:Int,add(x,1)),3); },0)",
        "state_answer(4,0)",
    );
}

#[test]
fn cli_runs_and_denotes_demo_and_interactive_surface_terms() {
    for args in [
        vec!["run", "examples/surface-syntax.soda"],
        vec!["denote", "examples/surface-syntax.soda"],
        vec![
            "run",
            "examples/surface-syntax.soda",
            "apply(do State { x <- state_get; return add(x,1); },5)",
        ],
    ] {
        let output = support::run(&args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{text}");
        assert!(text.contains("state_answer("), "{text}");
    }
}

#[test]
fn grouped_datatypes_support_induction_and_existing_duplicate_checks() {
    let source = r#"module Grouped;
        datatype Nat { constructor zero(); constructor succ(Nat); }
        function plus(Nat, Nat): Nat;
        rule add_zero: plus(zero, ?m) => ?m;
        rule add_succ: plus(succ(?n), ?m) => succ(plus(?n, ?m));
        axiom plus_zero: plus(zero, ?m) = ?m;
        axiom plus_succ: plus(succ(?n), ?m) = succ(plus(?n, ?m));
        induction right_zero on ?n: Nat: plus(?n, zero) = ?n {
            case zero { plus_zero with {m = zero}; }
            case succ(k) {
                plus_succ with {n = ?k, m = zero};
                ih_k at [0] with {k = ?k};
            }
        }
        eval plus(succ(zero), succ(zero));
    "#;
    let output = support::stdin_command("prove", source, &[]);
    assert!(output.status.success(), "{}", support::stderr(&output));
    assert!(support::stdout(&output).contains("theorem right_zero"));
    let module = load_module_source("<grouped>", source, std::path::Path::new("."))
        .unwrap()
        .module;
    assert_both(&module, "plus(succ(zero), succ(zero))", "succ(succ(zero))");
    let duplicate = support::stdin_command(
        "check",
        "module Bad; datatype V { constructor v(); } constructor v(): V;",
        &[],
    );
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(support::stderr(&duplicate).contains("E1004"));
}

#[test]
fn typed_functions_support_guards_forward_recursion_higher_order_values_and_named_traces() {
    let source = r#"module Functions;
        function even(Int): Bool {
            case even_zero: (0) => true;
            case even_step: (?n) => odd(sub(?n, 1)) when gt(?n, 0);
        }
        function odd(Int): Bool {
            case odd_zero: (0) => false;
            case odd_step: (?n) => even(sub(?n, 1)) when gt(?n, 0);
        }
        function invoke(?f: Int -> Int, ?n: Int): Int => apply(?f, ?n);
        function main(): Int => invoke(bind(x: Int, add(@x, 1)), 41);
        eval even(4);
        eval main;
    "#;
    let module = load_module_source("<functions>", source, std::path::Path::new("."))
        .unwrap()
        .module;
    soda_sorts::validate_module(&module).unwrap();
    soda_verifier::verify_module(&module).unwrap();
    for n in 0..=5 {
        assert_both(
            &module,
            &format!("even({n})"),
            if n % 2 == 0 { "true" } else { "false" },
        );
    }
    assert_both(&module, "main", "42");
    let output = support::stdin_command("run", source, &["--trace"]);
    assert!(output.status.success(), "{}", support::stderr(&output));
    let text = support::stdout(&output);
    assert!(text.contains("even_step"), "{text}");
    assert!(text.contains("invoke"), "{text}");
    let bad = support::stdin_command(
        "check",
        "module Bad; function f(Int): Int { case wrong_result: (?x) => false; }",
        &[],
    );
    assert_eq!(bad.status.code(), Some(2));
    let text = support::stderr(&bad);
    assert!(text.contains("rule `wrong_result`"), "{text}");
    assert!(text.contains("expected `Int`, found `Bool`"), "{text}");
    let free = support::stdin_command("check", "module Bad; function f(?x: Int): Int => ?y;", &[]);
    assert_eq!(free.status.code(), Some(2));
}
