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

