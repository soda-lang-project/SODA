use std::io::Write;
use std::process::{Output, Stdio};

pub mod support;
use support::{command, run as soda};

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn typed_judgments_and_binders_execute_from_files_and_interactive_inputs() {
    for (args, expected) in [
        (
            vec!["infer", "examples/typed-judgments.soda"],
            "meaning(lit(6), empty, 6)",
        ),
        (
            vec![
                "infer",
                "examples/typed-judgments.soda",
                "empty |- lit(4) : int_type",
            ],
            "typing(empty, lit(4), int_type)",
        ),
        (
            vec![
                "infer",
                "examples/typed-judgments.soda",
                "empty ⊢ lit(4) : int_type",
            ],
            "typing(empty, lit(4), int_type)",
        ),
        (
            vec![
                "infer",
                "examples/typed-judgments.soda",
                r"\llbracket lit(9) \rrbracket empty = ?v",
            ],
            "meaning(lit(9), empty, 9)",
        ),
        (
            vec![
                "run",
                "examples/bindings.soda",
                "offset(lambda y: Int. add(@y, 42))",
                "--strategy",
                "outermost",
            ],
            "result: 42",
        ),
        (vec!["run", "examples/bindings.soda"], "result: 42"),
        (
            vec!["infer", "examples/side-conditions.soda"],
            "positive(number(4))",
        ),
    ] {
        let output = soda(&args);

        assert!(output.status.success(), "{}", text(&output));
        assert!(text(&output).contains(expected), "{}", text(&output));
    }

    let output = soda(&[
        "run",
        "examples/bindings.soda",
        "apply(bind(x: Int, add(@x, 1)), false)",
    ]);

    assert_eq!(output.status.code(), Some(2));
    let external_binder = soda(&[
        "run",
        "examples/arithmetic.soda",
        "apply(bind(x: Int, @x), false)",
    ]);

    assert_eq!(external_binder.status.code(), Some(2));
    let output = soda(&[
        "infer",
        "examples/side-conditions.soda",
        "positive(number(0))",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("0 answers, complete"));
}

#[test]
fn induction_and_exact_finite_domains_have_checked_results() {
    let output = soda(&["prove", "examples/induction.soda"]);

    assert!(output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("theorem right_zero: 2 checked constructor cases"));
    assert!(text(&output).contains("theorem boxed_right_zero"));
    let output = soda(&["domains", "examples/domains.soda", "--trace"]);

    assert!(output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("domain Functions: 11 elements"));
    assert!(text(&output).contains("lfp(constant_true) = true"));
    assert!(text(&output).contains("bottom <= true"));
    let output = soda(&["domains", "examples/domains.soda", "negate(true)"]);

    assert!(output.status.success());
    assert!(text(&output).contains("negate(true) = false"));
    let output = soda(&[
        "correspond",
        "examples/correspondence.soda",
        "negate(meaning)",
    ]);

    assert!(output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("all 3 represented domain values"));
    let output = soda(&[
        "correspond",
        "examples/correspondence.soda",
        "negate(meaning)",
        "--fuel",
        "0",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("correspondence not established"));
}

#[test]
fn notation_output_and_named_premises_are_visible() {
    let unicode = soda(&[
        "infer",
        "examples/typed-judgments.soda",
        "--notation",
        "unicode",
        "--trace",
    ]);

    assert!(unicode.status.success(), "{}", text(&unicode));
    assert!(text(&unicode).contains('⊢'));
    assert!(text(&unicode).contains('⟦'));
    assert!(text(&unicode).contains("left:"));
    let ascii = soda(&[
        "infer",
        "examples/typed-judgments.soda",
        "--notation",
        "ascii",
    ]);

    assert!(ascii.status.success());
    assert!(text(&ascii).contains("|-"));
    assert!(!text(&ascii).contains('⊢'));
}

#[test]
fn source_stdin_rejects_unsound_typed_rules_and_circular_induction() {
    let invalid = [
        "module Bad; sort Expr; constructor lit(Int): Expr; function f(Expr): Expr; rule bad: f(lit(?n)) => lit(true);",
        "module Bad; datatype Nat; constructor zero(): Nat; rule bad: zero => zero;",
        "module Bad; eval apply(bind(x: Int, @x), false);",
        "module Bad; datatype Nat; constructor zero(): Nat; constructor succ(Nat): Nat; function p(Nat): Bool; axiom base: p(zero) = true; induction circular on ?n: Nat: p(?n) = true { case zero { base; } case succ(k) { ih_k with {k = succ(?k)}; } }",
    ];

    for source in invalid {
        let mut child = command()
            .args(["check", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();

        assert_eq!(output.status.code(), Some(2), "{}", text(&output));
    }
}
