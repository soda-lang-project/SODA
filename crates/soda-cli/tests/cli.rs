pub mod support;
use support::{run as soda, stdout};

#[test]
fn shipped_examples_are_executable() {
    for (file, expected) in [
        ("examples/arithmetic.soda", "720"),
        (
            "examples/naturals.soda",
            "succ(succ(succ(succ(succ(succ(zero))))))",
        ),
        ("examples/imperative.soda", "result(bind(x, 3"),
    ] {
        let output = soda(&["compare", file]);

        assert!(
            output.status.success(),
            "{file}: {} {}",
            stdout(&output),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(stdout(&output).contains(expected));
        assert!(!stdout(&output).contains("not established"));
    }

    let output = soda(&["run", "examples/combinators.soda"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("result: 42"));
}

#[test]
fn proofs_and_relational_derivations_check() {
    let output = soda(&["prove", "examples/arithmetic.soda"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("2 checked proofs"));
    let output = soda(&["infer", "examples/relations.soda", "--trace"]);

    assert!(
        output.status.success(),
        "{} {}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("sum(succ(succ(zero)), succ(zero), succ(succ(succ(zero))))"));
    assert!(stdout(&output).contains("2 answers, complete"));
    assert!(stdout(&output).contains("by type_if"));
    let output = soda(&["infer", "examples/hoare.soda", "--trace"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("by hoare_assign"));
    assert!(stdout(&output).contains("by hoare_sequence"));
}

#[test]
fn complete_and_truncated_nondeterministic_exploration() {
    let output = soda(&["explore", "examples/concurrency.soda", "--trace"]);

    assert!(
        output.status.success(),
        "{} {}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("complete exploration"));
    assert!(stdout(&output).contains("state(free, done, done)"));
    let output = soda(&["explore", "examples/concurrency.soda", "--states", "1"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("incomplete exploration"));
}

