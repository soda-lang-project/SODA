pub mod support;
use support::{command, stderr, stdin_command as stdin, stdout};

#[test]
fn every_human_command_uses_the_shared_palette_and_plain_or_json_modes_stay_clean() {
    let cases: &[(&[&str], &str)] = &[
        (
            &["run", "examples/arithmetic.soda", "add(1,2)", "--trace"],
            "operational evaluation",
        ),
        (
            &["eval", "examples/arithmetic.soda", "add(1,2)"],
            "normal form reached",
        ),
        (
            &["denote", "examples/arithmetic.soda", "add(1,2)", "--trace"],
            "compositional evaluation",
        ),
        (
            &["compare", "examples/arithmetic.soda", "add(1,2)", "--trace"],
            "evaluators agree on this input",
        ),
        (&["check", "examples/arithmetic.soda"], "module checked"),
        (
            &["prove", "examples/induction.soda", "--trace"],
            "induction checked",
        ),
        (
            &["infer", "examples/typed-judgments.soda", "--trace"],
            "derivation checked",
        ),
        (
            &["explore", "examples/concurrency.soda", "--trace"],
            "complete exploration",
        ),
        (
            &["analyze", "examples/naturals.soda"],
            "syntactic analysis complete",
        ),
        (
            &["domains", "examples/domains.soda", "--trace"],
            "least fixed point established",
        ),
        (
            &[
                "correspond",
                "examples/correspondence.soda",
                "negate(meaning)",
            ],
            "finite correspondence established",
        ),
    ];

    for (args, expected) in cases {
        for options in [
            vec!["--color", "always"],
            vec!["--color", "never"],
            vec!["--color", "always", "--diagnostic-format", "json"],
        ] {
            let output = command().args(*args).args(&options).output().unwrap();
            let text = stdout(&output);

            assert!(
                output.status.success(),
                "{args:?}: {text}{}",
                stderr(&output)
            );
            assert!(text.contains(expected), "{args:?}: {text}");
            let colored = options.len() == 2 && options[1] == "always";

            assert_eq!(
                text.contains('\x1b'),
                colored,
                "{args:?} {options:?}: {text:?}"
            );
            assert!(!text.contains("[NormalForm"), "{text}");
            if colored {
                assert!(text.contains("\x1b[96m"), "{args:?}: {text:?}");
                assert!(
                    !text.contains("\x1b[34m") && !text.contains("\x1b[90m"),
                    "{args:?}: {text:?}"
                );
            } else {
                assert_panels(&text, "+-- ", "|", "`--");
            }

            if options.len() > 2 {
                assert_eq!(stderr(&output), "{\"version\":1,\"diagnostics\":[]}\n");
            }
        }
    }
}

fn assert_panels(text: &str, opening: &str, gutter: &str, closing: &str) {
    let mut open = false;
    let mut panels = 0;

    for line in text.lines() {
        if line.starts_with(opening) {
            assert!(!open, "nested or unclosed panel: {text}");
            open = true;
            panels += 1;
        } else if line == closing {
            assert!(open, "closing a panel that was not opened: {text}");
            open = false;
        } else if open {
            assert!(
                line.starts_with(gutter),
                "missing gutter on {line:?}: {text}"
            );
        }
    }

    assert!(!open && panels > 0, "no complete panels: {text}");
}

