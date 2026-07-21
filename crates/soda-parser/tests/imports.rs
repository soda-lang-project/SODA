use soda_ast::Term;
use soda_parser::{load_module, load_module_source, parse_term};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "soda-import-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, source).unwrap();
        path
    }
    fn load(&self, name: &str) -> soda_parser::LoadedModule {
        load_module(self.0.join(name)).unwrap()
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn imports_are_header_directives_and_do_not_confuse_constructor_names_in_notation() {
    let workspace = Workspace::new();
    let source = r#"module Legacy;
        judgment triple(E,E,E);
        notation triple "{ $0 } $1 { $2 }" ascii "{| $0 |} $1 {| $2 |}";
        query { a } import { b };
    "#;
    let loaded = load_module_source("<stdin>", source, &workspace.0).unwrap();
    assert_eq!(
        loaded.module.queries[0],
        parse_term("triple(a, import, b)").unwrap()
    );
    let late = load_module_source(
        "<stdin>",
        "module Late; eval 42; import soda.collections;",
        &workspace.0,
    )
    .unwrap_err();
    assert!(late.message.contains("before local declarations"), "{late}");
}

#[test]
fn every_standard_library_is_bundled_and_independent_of_working_directory() {
    let workspace = Workspace::new();
    for (name, symbol) in [
        ("soda.effects.state", "state_bind"),
        ("soda.effects.exceptions", "exception_bind"),
        ("soda.probability", "prob_bind"),
        ("soda.collections", "list_map"),
        ("soda.collections.sets", "set_union"),
        ("soda.languages.arithmetic", "lit"),
        ("soda.data.nat", "nat_add"),
        ("soda.math.rational", "radd"),
        ("soda.prelude", "nat_mul"),
    ] {
        let loaded = load_module_source(
            "<stdin>",
            &format!("module Root; import {name};"),
            &workspace.0,
        )
        .unwrap();
        assert!(loaded.module.signatures.iter().any(|s| s.name == symbol));
        assert!(
            loaded
                .sources
                .iter()
                .any(|source| source.name == format!("<{name}>"))
        );
        assert_eq!(loaded.sources[loaded.root].name, "<stdin>");
    }
}

#[test]
fn relative_and_dotted_imports_resolve_from_each_importing_file() {
    let workspace = Workspace::new();
    workspace.write("lib/value.soda", "module Value; rule value: value => 9;");
    workspace.write(
        "lib/wrapper.soda",
        "module Wrapper; import value; rule wrapper: wrapper => value;",
    );
    workspace.write(
        "root.soda",
        "module Root; import lib.wrapper; import \"lib/value.soda\"; eval wrapper;",
    );
    let loaded = workspace.load("root.soda");
    assert_eq!(loaded.module.rules.len(), 2);
    assert_eq!(
        loaded.module.evaluations,
        vec![parse_term("wrapper").unwrap()]
    );
    assert_eq!(loaded.sources.len(), 3);
    assert!(loaded.sources[0].name.ends_with("value.soda"));
    assert!(loaded.sources[1].name.ends_with("wrapper.soda"));
}

