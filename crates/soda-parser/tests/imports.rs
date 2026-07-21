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

#[test]
fn diamonds_and_repeated_imports_deduplicate_shared_definitions() {
    let workspace = Workspace::new();
    workspace.write(
        "shared.soda",
        "module Shared; rule shared: shared => 1; eval 100; query ignored;",
    );
    workspace.write(
        "left.soda",
        "module Left; import shared; rule left: left => shared;",
    );
    workspace.write(
        "right.soda",
        "module Right; import shared; rule right: right => shared;",
    );
    workspace.write(
        "root.soda",
        "module Root; import left; import right; import shared; eval 7; query root_query;",
    );
    let loaded = workspace.load("root.soda");
    assert_eq!(loaded.sources.len(), 4);
    assert_eq!(
        loaded
            .module
            .rules
            .iter()
            .filter(|r| r.name == "shared")
            .count(),
        1
    );
    assert_eq!(loaded.module.rules.len(), 3);
    assert_eq!(loaded.module.evaluations, vec![Term::Int(7)]);
    assert_eq!(
        loaded.module.queries,
        vec![parse_term("root_query").unwrap()]
    );
    assert_eq!(loaded.root, 3);
    assert!(loaded.module.imports.is_empty());
}

#[test]
fn root_source_strings_use_explicit_base_for_local_imports() {
    let workspace = Workspace::new();
    workspace.write("support.soda", "module Support; rule answer: answer => 42;");
    let loaded = load_module_source(
        "<stdin>",
        "module Stdin; import support; eval answer;",
        &workspace.0,
    )
    .unwrap();
    assert_eq!(loaded.module.name, "Stdin");
    assert_eq!(loaded.module.rules.len(), 1);
    assert_eq!(
        loaded.sources[loaded.root].text,
        "module Stdin; import support; eval answer;"
    );
}

#[test]
fn imported_records_monads_and_notations_are_available_while_parsing_root() {
    let workspace = Workspace::new();
    workspace.write(
        "prelude.soda",
        r#"module Prelude;
        record Store = store {value: Int, valid: Bool};
        sort Computation;
        function unit(Int): Computation;
        function chain(Computation, Int -> Computation): Computation;
        monad M {bind = chain; return = unit;};
        judgment reduces(Int, Int);
        notation reduces "$0 → $1" ascii "$0 -> $1";
    "#,
    );
    workspace.write("root.soda", "module Root; import prelude; eval store {valid: true, value: 8}; eval do M { x <- unit(1); return x; }; query 1 → 2;");
    let loaded = workspace.load("root.soda");
    assert_eq!(
        loaded.module.evaluations[0],
        parse_term("store(8, true)").unwrap()
    );
    assert_eq!(
        loaded.module.evaluations[1],
        parse_term("chain(unit(1), bind(x: Int, unit(@x)))").unwrap()
    );
    assert_eq!(
        loaded.module.queries,
        vec![parse_term("reduces(1, 2)").unwrap()]
    );
    assert_eq!(loaded.module.records.len(), 1);
    assert_eq!(loaded.module.monads.len(), 1);
    assert_eq!(loaded.module.notations.len(), 1);
}

#[test]
fn cycle_errors_identify_import_chain_and_source() {
    let workspace = Workspace::new();
    workspace.write("a.soda", "module A; import b;");
    workspace.write("b.soda", "module B; import a;");
    let error = load_module(workspace.0.join("a.soda")).unwrap_err();
    assert!(error.message.contains("cyclic imports"), "{error:?}");
    assert!(error.message.contains("a.soda") && error.message.contains("b.soda"));
    assert!(error.source.ends_with("a.soda"));
    assert_eq!(error.text.as_deref(), Some("module A; import b;"));
}

#[test]
fn missing_import_and_dependency_parse_errors_retain_the_right_source() {
    let workspace = Workspace::new();
    let root = workspace.write("root.soda", "module Root; import missing;");
    let missing = load_module(&root).unwrap_err();
    assert!(missing.message.contains("cannot import `missing`"));
    assert_eq!(Path::new(&missing.source), root);
    let span = missing.span.unwrap();
    assert_eq!(
        &missing.text.unwrap()[span.start..span.end],
        "import missing;"
    );

    workspace.write("broken.soda", "module Broken; rule invalid: =>;");
    workspace.write("root.soda", "module Root; import broken;");
    let broken = load_module(&root).unwrap_err();
    assert!(broken.source.ends_with("broken.soda"));
    assert_eq!(
        broken.text.as_deref(),
        Some("module Broken; rule invalid: =>;")
    );
    assert!(broken.parse.is_some(), "{broken:?}");
}

#[test]
fn duplicate_definition_names_across_modules_are_rejected() {
    let workspace = Workspace::new();
    workspace.write("a.soda", "module A; rule collision: a => 1;");
    workspace.write("b.soda", "module B; rule collision: b => 2;");
    workspace.write("root.soda", "module Root; import a; import b;");
    let error = load_module(workspace.0.join("root.soda")).unwrap_err();
    assert!(error.message.contains("collision"), "{error:?}");
    assert!(error.message.contains("duplicate"), "{error:?}");
}

#[test]
fn imported_and_local_definition_names_cannot_shadow_each_other() {
    let workspace = Workspace::new();
    workspace.write("dependency.soda", "module Dependency; rule shared: a => 1;");
    workspace.write(
        "root.soda",
        "module Root; import dependency; rule shared: b => 2;",
    );
    let error = load_module(workspace.0.join("root.soda")).unwrap_err();
    assert!(error.source.ends_with("root.soda"));
    assert!(error.message.contains("shared"), "{error:?}");
    assert!(error.message.contains("duplicate"), "{error:?}");
    assert!(error.parse.is_some(), "{error:?}");
}

