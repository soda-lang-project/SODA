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

