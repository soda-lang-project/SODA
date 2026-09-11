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

#[test]
fn failure_and_limit_exit_codes_are_not_success() {
    let negative = soda(&["run", "examples/arithmetic.soda", "-42"]);

    assert!(negative.status.success());
    assert!(stdout(&negative).contains("result: -42"));
    assert_eq!(
        soda(&[
            "run",
            "examples/arithmetic.soda",
            "factorial(6)",
            "--fuel",
            "0"
        ])
        .status
        .code(),
        Some(1)
    );
    assert_eq!(
        soda(&["run", "examples/arithmetic.soda", "div(1, 0)"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        soda(&["run", "examples/arithmetic.soda", "?free"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        soda(&["check", "does-not-exist.soda"]).status.code(),
        Some(2)
    );
    assert_eq!(
        soda(&["run", "examples/arithmetic.soda", "--depth", "999999"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        soda(&["unknown", "examples/arithmetic.soda"]).status.code(),
        Some(2)
    );
}

#[test]
fn bounded_comparison_does_not_claim_universal_equivalence() {
    let output = soda(&[
        "compare",
        "examples/arithmetic.soda",
        "factorial(6)",
        "--fuel",
        "1",
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains("not established"));
    assert!(stdout(&soda(&["--help"])).contains("not universal equivalence"));
}

#[test]
fn clap_help_and_global_options_work_before_and_after_commands() {
    for args in [vec![], vec!["--help"], vec!["run", "--help"]] {
        let output = soda(&args);

        assert!(output.status.success());
        assert!(stdout(&output).contains("--fuel"));

        if args.len() <= 1 {
            assert!(stdout(&output).contains("not universal equivalence"));
        }
    }

    let version = soda(&["--version"]);

    assert!(version.status.success());
    assert!(stdout(&version).contains(concat!("soda ", env!("CARGO_PKG_VERSION"))));
    let output = soda(&[
        "--diagnostic-format=json",
        "--fuel=10000",
        "run",
        "examples/arithmetic.soda",
        "-42",
        "--notation=ascii",
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("result: -42"));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "{\"version\":1,\"diagnostics\":[]}\n"
    );
}

