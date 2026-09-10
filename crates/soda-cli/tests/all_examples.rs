pub mod support;
use std::path::{Path, PathBuf};
use support::{run, stderr, stdout};

fn examples(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            examples(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "soda")
        {
            files.push(path);
        }
    }
}

#[test]
fn every_example_checks_and_every_shipped_input_executes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    examples(&root.join("examples"), &mut files);
    files.sort();
    assert!(!files.is_empty());
    let mut valid = 0;
    let mut invalid = 0;
    let mut commands = 0;
    let mut failures = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        if relative.starts_with("examples/diagnostics/") {
            let (command, expected_exit, message) =
                match file.file_stem().unwrap().to_str().unwrap() {
                    "bad-proof" => ("prove", 2, "error[E2001]"),
                    "missing-semicolon" => ("check", 2, "expected ';'"),
                    "multiple-errors" | "wrong-sort" => ("check", 2, "error[E1001]"),
                    "nonmonotone" => ("check", 2, "error[E3002]"),
                    "runtime" => ("run", 1, "error[E4001]"),
                    "unknown-symbol" => ("check", 2, "error[E1002]"),
                    name => panic!("classify the new diagnostic fixture {name}"),
                };
            let output = run(&[command, &relative]);
            if output.status.code() != Some(expected_exit) || !stderr(&output).contains(message) {
                failures.push(format!(
                    "{command} {relative}: expected exit {expected_exit} and {message}: {}{}",
                    stdout(&output),
                    stderr(&output)
                ));
            }
            println!("EXPECTED FAILURE: {relative} ({command}, exit {expected_exit})");
            invalid += 1;
            commands += 1;
            continue;
        }
        let module =
            soda_parser::load_module(&file).unwrap_or_else(|error| panic!("{relative}: {error}"));
        let mut checks = vec![("check", None)];
        if !module.module.evaluations.is_empty() {
            // SKI needs lazy rewriting; concurrency inputs are graph roots.

            let command = match relative.as_str() {
                "examples/combinators.soda" => "run",
                "examples/concurrency.soda" => "explore",
                _ => "compare",
            };
            checks.push((command, None));
        }
        if !module.module.queries.is_empty() {
            checks.push(("infer", None));
        }
        if !module.module.domains.is_empty() {
            checks.push(("domains", None));
        }
        if relative == "examples/correspondence.soda" {
            checks.push(("correspond", Some("negate(meaning)")));
        }
        for (command, term) in checks {
            let mut args = vec![command, relative.as_str()];
            if let Some(term) = term {
                args.push(term);
            }
            let output = run(&args);
            if !output.status.success() {
                failures.push(format!(
                    "{command} {relative}: {}{}",
                    stdout(&output),
                    stderr(&output)
                ));
            }
            println!(
                "CHECKED: {relative} ({command}, exit {:?})",
                output.status.code()
            );
            commands += 1;
        }
        valid += 1;
    }
    println!(
        "Audited {valid} valid examples and {invalid} intentional failures with {commands} CLI invocations"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
