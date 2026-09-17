pub mod support;
use support::{run, stderr, stdin_command};
fn stdin(source: &str, extra: &[&str]) -> std::process::Output {
    stdin_command("check", source, extra)
}

#[test]
fn sort_diagnostics_point_to_the_argument_and_relate_its_signature() {
    let output = run(&["check", "examples/diagnostics/wrong-sort.soda"]);
    let text = stderr(&output);

    assert_eq!(output.status.code(), Some(2));
    assert!(text.contains("error[E1001]"), "{text}");
    assert!(text.contains("wrong-sort.soda:6:14"));
    assert!(text.contains("^^^^^ expected `Int`, found `Bool`"));
    assert!(text.contains("`literal` is declared here"));
    assert!(!text.contains('\x1b'));
}

#[test]
fn spelling_and_missing_terminator_fixes_show_correct_source_previews() {
    let output = run(&["check", "examples/diagnostics/unknown-symbol.soda"]);
    let text = stderr(&output);

    assert!(text.contains("did you mean `literal`?"));
    assert!(text.contains("6 | eval literal(42);"), "{text}");
    let output = run(&["check", "examples/diagnostics/missing-semicolon.soda"]);
    let text = stderr(&output);

    assert!(text.contains("expected ';'"));
    assert!(
        text.contains("3 | rule identity: identity(?x) => ?x;"),
        "{text}"
    );
    let output = stdin(
        "module M; function f(Int): Booll;",
        &["--diagnostic-format", "json"],
    );
    let text = stderr(&output);

    assert!(text.contains("\"replacement\":\"Bool\""));
    assert!(text.contains("\"start\":27,\"end\":32"), "{text}");
}

#[test]
fn independent_errors_are_ordered_and_the_report_has_an_error_cap() {
    let output = run(&["check", "examples/diagnostics/multiple-errors.soda"]);
    let text = stderr(&output);

    assert_eq!(text.matches("error[").count(), 3);
    assert!(text.find(":6:14").unwrap() < text.find(":7:6").unwrap());
    assert!(text.find(":7:6").unwrap() < text.find(":8:10").unwrap());
    let source = format!(
        "module M; function f(Int): Int; {}",
        "eval f(false);".repeat(23)
    );
    let output = stdin(&source, &[]);
    let text = stderr(&output);

    assert_eq!(text.matches("error[E1001]").count(), 20);
    assert!(text.contains("3 further sort errors were omitted"));
}

#[test]
fn proof_diagnostics_explain_the_wrong_instantiation_and_reference_the_axiom() {
    let output = run(&["prove", "examples/diagnostics/bad-proof.soda"]);
    let text = stderr(&output);

    assert!(text.contains("error[E2001]"));
    assert!(text.contains("`?x` has the wrong instance here"), "{text}");
    assert!(text.contains("expected: add(3, 0)"));
    assert!(text.contains("found:    add(4, 0)"));
    assert!(text.contains("instantiate `?x` with `3`"));
    assert!(text.contains("the referenced equation is declared here"));
}

#[test]
fn runtime_and_resource_failures_are_source_aware_and_keep_exit_codes() {
    let error = run(&["run", "examples/diagnostics/runtime.soda"]);
    let text = stderr(&error);

    assert_eq!(error.status.code(), Some(1));
    assert!(text.contains("error[E4001]"));
    assert!(text.contains("div by zero"));
    assert!(text.contains("last applied rule"), "{text}");
    let limited = run(&[
        "run",
        "examples/arithmetic.soda",
        "factorial(6)",
        "--fuel",
        "0",
    ]);
    let text = stderr(&limited);

    assert_eq!(limited.status.code(), Some(1));
    assert!(text.contains("warning[W4002]"));
    assert!(text.contains("<term argument>:1:1"));
    assert!(text.contains("does not establish divergence"));
}

#[test]
fn machine_output_is_one_ansi_free_document_even_with_multiple_errors_or_color() {
    let output = run(&[
        "check",
        "examples/diagnostics/multiple-errors.soda",
        "--diagnostic-format",
        "json",
        "--color",
        "always",
    ]);
    let text = stderr(&output);

    assert!(text.starts_with("{\"version\":1,\"diagnostics\":["));
    assert!(text.ends_with("]}\n"));
    assert_eq!(text.matches("\"severity\":\"error\"").count(), 3);
    assert!(text.contains("\"source\":\"examples/diagnostics/multiple-errors.soda\""));
    assert!(!text.contains('\x1b'));
    let output = run(&[
        "check",
        "examples/arithmetic.soda",
        "--diagnostic-format",
        "json",
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "{\"version\":1,\"diagnostics\":[]}\n");
}

#[test]
fn colors_ascii_and_stdin_locations_work_without_terminal_assumptions() {
    let source = "module Πρόβλημα;\nfunction f(Int): Int;\neval f(false);\n";
    let output = stdin(source, &["--diagnostic-style", "ascii", "--color", "never"]);
    let text = stderr(&output);

    assert!(text.contains("--> <stdin>:3:8"), "{text}");
    assert!(!text.contains('│'));
    assert!(!text.contains('\x1b'));
    let output = stdin(source, &["--color", "always"]);

    assert!(stderr(&output).contains("\x1b["));
    assert!(stderr(&output).contains("[E1001]"));
    let output = run(&[
        "run",
        "examples/bindings.soda",
        "apply(bind(x: Int, @x), false)",
    ]);
    let text = stderr(&output);

    assert!(text.contains("<term argument>"));
    assert!(text.contains("expected `Int`, found `Bool`"), "{text}");
}

#[test]
fn inference_model_domain_and_induction_failures_get_specific_guidance() {
    let output = run(&[
        "infer",
        "examples/side-conditions.soda",
        "positive(number(0))",
    ]);

    assert!(stderr(&output).contains("error[E4102]"));
    let output = run(&["infer", "examples/relations.soda", "--fuel", "0"]);

    assert!(stderr(&output).contains("warning[W4101]"));
    let output = run(&["explore", "examples/concurrency.soda", "--states", "1"]);

    assert!(stderr(&output).contains("warning[W4301]"));
    let output = run(&["check", "examples/diagnostics/nonmonotone.soda"]);

    assert!(stderr(&output).contains("error[E3002]"));
    assert!(stderr(&output).contains("bottom <= false"));
    assert!(!stderr(&output).contains("broken: broken:"));
    let source = "module M; datatype Nat; constructor zero(): Nat; constructor succ(Nat): Nat; function f(Nat): Nat; induction p on ?n: Nat: f(?n) = ?n { case zero {} }";
    let output = stdin(source, &[]);

    assert!(stderr(&output).contains("error[E2101]"));
    assert!(stderr(&output).contains("missing cases: succ"));
}

