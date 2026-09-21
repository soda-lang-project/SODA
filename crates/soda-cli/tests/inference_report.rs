pub mod support;
use support::{stderr, stdout};
fn infer(source: &str, options: &[&str]) -> std::process::Output {
    support::stdin_command("infer", source, options)
}

#[test]
fn successful_queries_have_goals_and_checked_derivations_without_repeated_ground_answers() {
    let output = infer(
        "module Suite; infer fact: valid; query valid;",
        &["--color", "never"],
    );
    let text = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(text.contains("1 goal in Suite"), "{text}");
    assert!(text.contains("goal 1/1\n|\n| valid"), "{text}");
    assert!(
        text.contains("1 answer, complete search (1 attempt)"),
        "{text}"
    );
    assert!(
        text.contains("1 derivation checked\n1 goal resolved;"),
        "{text}"
    );
    assert_eq!(
        text.matches("valid").count(),
        1,
        "ground answers should not repeat without --trace"
    );
    assert!(!text.contains('\x1b'));

    for label in ["PASS", "FAIL", "UNKNOWN", "passed", "failed"] {
        assert!(!text.contains(label), "{text}");
    }
}

#[test]
fn unicode_and_ascii_traces_preserve_labeled_tree_edges() {
    let source = "module Tree; infer first: a; infer second: b; infer root: both if left: a, right: b; query both;";

    for (notation, left, right) in [
        ("unicode", "├─ left:", "└─ right:"),
        ("ascii", "+- left:", "`- right:"),
    ] {
        let output = infer(
            source,
            &["--trace", "--notation", notation, "--color", "never"],
        );
        let text = stdout(&output);

        assert!(output.status.success(), "{}", stderr(&output));
        assert!(text.contains(left) && text.contains(right), "{text}");
        assert!(
            text.contains("by first") && text.contains("by second"),
            "{text}"
        );
        assert_eq!(text.matches("by root").count(), 1);
        assert!(!text.contains('\x1b'));
    }
}

#[test]
fn completed_search_and_incomplete_search_keep_goals_distinct() {
    let source = "module Mixed; infer fact: valid; query valid; query absent;";
    let output = infer(source, &["--color", "never"]);
    let text = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(text.contains("goal 1/2\n|\n| valid"), "{text}");
    assert!(text.contains("goal 2/2\n|\n| absent"), "{text}");
    assert!(text.contains("no derivation found"));
    assert!(
        text.contains("1 goal resolved; 1 goal without a derivation"),
        "{text}"
    );
    assert!(stderr(&output).contains("error[E4102]"));
    let output = infer(source, &["--fuel", "0", "--color", "never"]);
    let text = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(text.matches("goal remains unresolved").count(), 2, "{text}");
    assert!(
        text.contains("0 goals resolved; 2 unresolved goals"),
        "{text}"
    );
    assert!(text.contains("2 incomplete searches"));
    assert!(text.contains("search limit reached; goal remains unresolved"));
    assert!(!text.contains("no derivation found"));
    assert!(stderr(&output).contains("warning[W4101]"));
}

#[test]
fn an_answer_is_established_even_when_further_enumeration_is_incomplete() {
    let output = infer(
        "module Partial; infer fact: valid; infer again: valid if valid; query valid;",
        &["--fuel", "1", "--color", "never"],
    );
    let text = stdout(&output);

    assert!(output.status.success(), "{text}{}", stderr(&output));
    assert!(text.contains("goal 1/1\n|\n| valid"), "{text}");
    assert!(text.contains("1 answer, incomplete search"), "{text}");
    assert!(
        text.contains("1 incomplete search; additional answers may remain undiscovered"),
        "{text}"
    );
    assert!(stderr(&output).contains("warning[W4101]"));
}

#[test]
fn forced_color_applies_to_successful_stdout_and_json_mode_remains_ansi_free() {
    let source = "module Color; infer fact: valid; query valid;";
    let output = infer(source, &["--color", "always"]);
    let text = stdout(&output);

    assert!(output.status.success());
    assert!(text.contains("\x1b[92mderivation checked"), "{text:?}");
    let output = infer(source, &[]);

    assert!(
        !stdout(&output).contains('\x1b'),
        "captured auto output must remain plain"
    );
    let output = infer(
        source,
        &["--color", "always", "--diagnostic-format", "json"],
    );

    assert!(output.status.success());
    assert!(!stdout(&output).contains('\x1b'));
    assert_eq!(stderr(&output), "{\"version\":1,\"diagnostics\":[]}\n");
}

#[test]
fn derivation_counts_are_distinct_from_resolved_goal_counts() {
    let output = infer(
        "module Alternatives; infer first: choice(one); infer second: choice(two); query choice(?x);",
        &["--trace", "--color", "never"],
    );
    let text = stdout(&output);

    assert!(output.status.success(), "{text}{}", stderr(&output));
    assert!(
        text.contains("derivation 1/2 checked") && text.contains("derivation 2/2 checked"),
        "{text}"
    );
    assert!(
        text.contains("2 derivations checked\n1 goal resolved;"),
        "{text}"
    );
    assert!(text.contains("by first") && text.contains("by second"));
}

#[test]
fn incomplete_conditions_do_not_misreport_a_resource_limit() {
    let output = infer(
        "module Condition; infer guarded: valid when eq(div(1, 0), 1); query valid;",
        &["--color", "never"],
    );
    let text = stdout(&output);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        text.contains("search incomplete; goal remains unresolved"),
        "{text}"
    );
    assert!(!text.contains("search limit reached"), "{text}");
    assert!(!text.contains("no derivation found"), "{text}");
}

