pub mod evaluation;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

pub fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_soda"));
    command
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env("NO_COLOR", "1");

    command
}

pub fn run(args: &[&str]) -> Output {
    command().args(args).output().unwrap()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

pub fn stdin_command(name: &str, source: &str, extra: &[&str]) -> Output {
    let mut child = command()
        .args([name, "-"])
        .args(extra)
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

    child.wait_with_output().unwrap()
}
