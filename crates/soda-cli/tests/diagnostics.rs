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

