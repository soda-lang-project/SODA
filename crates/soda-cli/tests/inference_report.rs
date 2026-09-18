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

