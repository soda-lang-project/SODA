use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "soda-cli-import-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, source: &str) {
        std::fs::write(self.0.join(name), source).unwrap();
    }
    fn check(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_soda"))
            .args(["check"])
            .arg(self.0.join("main.soda"))
            .args(["--color", "never"])
            .output()
            .unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn imported_sort_errors_name_the_original_definition_and_file() {
    let files = Files::new();
    files.write("main.soda", "module Main;\nimport \"bad.soda\";\n");
    files.write(
        "bad.soda",
        "module Bad;\nfunction broken(Int): Int;\nrule wrong: broken(?x) => false;\n",
    );
    let output = files.check();
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(text.contains("bad.soda:3:"), "{text}");
    assert!(text.contains("rule `wrong`"), "{text}");
    assert!(text.contains("expected `Int`, found `Bool`"), "{text}");
}

#[test]
fn imported_certificate_errors_highlight_the_certificate_in_its_source() {
    let files = Files::new();
    files.write("main.soda", "module Main;\nimport \"bad.soda\";\n");
    files.write("bad.soda", "module Bad;\naxiom identity: f(?x) = ?x;\nproof wrong: f(1) = 2 { identity with {x = 1}; }\n");
    let output = files.check();
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(text.contains("bad.soda:3:"), "{text}");
    assert!(text.contains("could not verify `wrong`"), "{text}");
    assert!(text.contains("expected: 2"), "{text}");
    assert!(text.contains("found:    1"), "{text}");
}

#[test]
fn missing_import_points_at_the_import_declaration() {
    let files = Files::new();
    files.write("main.soda", "module Main;\nimport \"missing.soda\";\n");
    let output = files.check();
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(text.contains("E0102"), "{text}");
    assert!(text.contains("main.soda:2:"), "{text}");
    assert!(text.contains("cannot import `missing.soda`"), "{text}");
}

#[test]
fn all_syntax_features_work_together_in_the_language_tour() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/language-tour.soda");
    for command in ["check", "run", "denote", "infer"] {
        let output = Command::new(env!("CARGO_BIN_EXE_soda"))
            .arg(command)
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{command}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if command == "infer" {
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("observed(11, cons(15, cons(11, end)))")
            );
        }
    }
}

#[test]
fn syntax_only_modules_cannot_silently_ignore_unresolved_imports() {
    let module = soda_parser::parse_module("module Unlinked; import soda.collections;").unwrap();
    assert!(
        soda_sorts::validate_module(&module)
            .unwrap_err()
            .to_string()
            .contains("unresolved imports")
    );
    assert!(
        soda_verifier::validate_module(&module)
            .unwrap_err()
            .to_string()
            .contains("unresolved imports")
    );
}

#[test]
fn generated_rust_preserves_the_linked_language_tour_without_import_runtime() {
    let files = Files::new();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let loaded = soda_parser::load_module(workspace.join("examples/language-tour.soda")).unwrap();
    assert!(loaded.module.imports.is_empty());
    assert!(!loaded.module.records.is_empty());
    assert!(!loaded.module.monads.is_empty());
    assert!(
        loaded
            .module
            .clause_details
            .iter()
            .any(|detail| !detail.bindings.is_empty())
    );
    let source = format!(
        "{}\nfn main() {{ assert_eq!(format!(\"{{:?}}\", module()), {:?}); }}\n",
        soda_codegen::emit_rust(&loaded.module),
        format!("{:?}", loaded.module)
    );
    files.write("generated.rs", &source);
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let library = files.0.join("libsoda_ast.rlib");
    let ast = Command::new(&rustc)
        .args([
            "--edition=2024",
            "--crate-type=rlib",
            "--crate-name=soda_ast",
            "-Dwarnings",
        ])
        .arg(workspace.join("crates/soda-ast/src/lib.rs"))
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        ast.status.success(),
        "{}",
        String::from_utf8_lossy(&ast.stderr)
    );
    let executable = files
        .0
        .join(format!("generated{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new(&rustc)
        .args(["--edition=2024", "-Dwarnings", "--extern"])
        .arg(format!("soda_ast={}", library.display()))
        .arg(files.0.join("generated.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let restored = Command::new(executable).output().unwrap();
    assert!(
        restored.status.success(),
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
}
