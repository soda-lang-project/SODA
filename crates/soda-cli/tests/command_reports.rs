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

#[test]
fn evaluation_steps_and_certificates_show_colored_rule_provenance() {
    for args in [
        vec!["run", "examples/arithmetic.soda", "add(1,2)", "--trace"],
        vec!["prove", "examples/arithmetic.soda", "--trace"],
    ] {
        let output = command()
            .args(&args)
            .args(["--color", "always"])
            .output()
            .unwrap();
        let text = stdout(&output);

        assert!(output.status.success(), "{text}{}", stderr(&output));
        assert!(text.contains("\x1b[95mby "), "{text:?}");
        assert!(text.contains("step 1"), "{text}");
    }
}

#[test]
fn runtime_errors_and_limits_keep_their_distinct_semantic_outcomes() {
    for (term, extra, code, message, tone) in [
        (
            "factorial(6)",
            vec!["--fuel", "0"],
            "W4002",
            "reduction limit reached",
            "\x1b[93m",
        ),
        ("div(1,0)", vec![], "E4001", "evaluation error", "\x1b[91m"),
    ] {
        let output = command()
            .args(["run", "examples/arithmetic.soda", term, "--color", "always"])
            .args(extra)
            .output()
            .unwrap();
        let text = stdout(&output);

        assert_eq!(output.status.code(), Some(1));
        assert!(text.contains(&format!("{tone}{message}")), "{text:?}");
        assert!(!text.contains("normal form reached"));
        assert!(stderr(&output).contains(code));
    }
}

#[test]
fn comparison_counterexamples_are_not_reported_as_unresolved_obligations() {
    let source =
        "module Order; rule root: f(g) => 1; rule child: g => 2; rule other: f(2) => 3; eval f(g);";
    let output = stdin("compare", source, &["--color", "never"]);
    let text = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(text.contains("different normal forms"), "{text}");
    assert!(
        text.contains("0 comparisons established; 1 counterexample"),
        "{text}"
    );
    assert!(!text.contains("obligation unresolved"), "{text}");
    assert!(stderr(&output).contains("E4401"));
}

#[test]
fn truncated_exploration_and_correspondence_do_not_claim_a_complete_result() {
    let exploration = command()
        .args([
            "explore",
            "examples/concurrency.soda",
            "--states",
            "1",
            "--color",
            "always",
        ])
        .output()
        .unwrap();

    assert_eq!(exploration.status.code(), Some(1));
    assert!(stdout(&exploration).contains("\x1b[93mincomplete exploration"));
    assert!(stdout(&exploration).contains("1 partial graph"));
    let correspondence = command()
        .args([
            "correspond",
            "examples/correspondence.soda",
            "negate(meaning)",
            "--fuel",
            "0",
            "--color",
            "always",
        ])
        .output()
        .unwrap();

    assert_eq!(correspondence.status.code(), Some(1));
    assert!(stdout(&correspondence).contains("\x1b[93mcorrespondence not established"));
    assert!(!stdout(&correspondence).contains("counterexample found"));
}

#[test]
fn raw_rust_export_remains_unadorned_even_when_color_is_forced() {
    let output = command()
        .args(["emit-rust", "examples/arithmetic.soda", "--color", "always"])
        .output()
        .unwrap();
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(text.contains("pub fn module()"), "{text}");
    assert!(!text.contains('\x1b'));
    assert!(!text.contains("module checked"));
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

#[test]
fn unicode_panels_keep_their_gutters_across_every_trace_line() {
    let output = command()
        .args([
            "eval",
            "examples/arithmetic.soda",
            "add(1,2)",
            "--trace",
            "--notation",
            "unicode",
            "--color",
            "never",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert_panels(&text, "╭─ ", "│", "╰─");

    assert!(text.contains("│     → 3"), "{text}");
}

