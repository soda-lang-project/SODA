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

