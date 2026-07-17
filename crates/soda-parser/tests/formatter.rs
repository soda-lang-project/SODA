use soda_parser::{format_source, lexer::lex, load_module_source};

#[test]
fn formatting_preserves_comments_literals_and_is_idempotent() {
    let source = "module X;  \r\nproof p: f(1,2)=f(1,2) {\r\n# keep comment\r\n  }\r\nnotation j \"$0 // # { }\";\r\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("f(1, 2) = f(1, 2)"));
    assert!(formatted.contains("    # keep comment\r\n}"));
    assert!(formatted.contains("\"$0 // # { }\""));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn multiline_strings_are_opaque_and_bad_literals_are_rejected() {
    let source = "module X;\nnotation j \"$0\n   keep this   \n$1\";\n";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("\"$0\n   keep this   \n$1\""));
    assert!(format_source("module X; notation j \"unterminated").is_err());
}

#[test]
fn declarations_and_inline_comments_get_distinct_lines() {
    let source = "module X; rule r:f(1,2)=>if(true,1,2); // keep with r\nrule s:g=>3;eval f(1,2);";
    let formatted = format_source(source).unwrap();
    assert_eq!(
        formatted,
        "module X;\n\nrule r: f(1, 2) => if true then 1 else 2; // keep with r\n\nrule s: g => 3;\n\neval f(1, 2);\n"
    );
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn long_calls_break_arguments_and_indent_nested_calls() {
    let source = "module X; eval run_program(assign(freeze(2, undef), assign(sum(2, wrapping, reg(0), reg(0)), ret(2, reg(0)))), empty);";
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains("eval run_program(\n    assign("),
        "{formatted}"
    );
    assert!(formatted.contains("\n    empty\n);"), "{formatted}");
    assert!(formatted.lines().all(|line| line.chars().count() <= 100));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn alive2_has_wrapped_rules_correct_continuations_and_stable_layout() {
    let source = include_str!("../../../examples/alive2-integers.soda");
    let formatted = format_source(source).unwrap().replace("\r\n", "\n");
    assert!(
        formatted
            .contains("case divide_poison_divisor:\n        (?w, ?a, poison(?w))\n        => ub;")
    );
    assert!(formatted.contains(";\n    case power_succ:"));
    assert!(!formatted.contains("if("));
    assert!(formatted.contains("\n        when width_ok(?w);"));
    assert!(formatted.contains("}\n\nfunction "));
    assert!(formatted.lines().all(|line| line.chars().count() <= 100));
    assert_eq!(format_source(&formatted).unwrap(), formatted);
}

#[test]
fn every_example_and_library_preserves_tokens_linked_ast_and_comments() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for directory in [
        root.join("examples"),
        root.join("crates/soda-parser/stdlib"),
    ] {
        let mut files = Vec::new();
        source_files(&directory, &mut files);
        for path in files {
            let source = std::fs::read_to_string(&path).unwrap();
            let formatted =
                format_source(&source).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(
                format_source(&formatted).unwrap(),
                formatted,
                "{}",
                path.display()
            );
            let before = lex(&source)
                .unwrap()
                .into_iter()
                .map(|t| t.kind)
                .collect::<Vec<_>>();
            let after = lex(&formatted)
                .unwrap()
                .into_iter()
                .map(|t| t.kind)
                .collect::<Vec<_>>();
            if before != after {
                assert_eq!(
                    soda_parser::parse_module(&source).unwrap(),
                    soda_parser::parse_module(&formatted).unwrap(),
                    "{}",
                    path.display()
                );
            }
            let name = path.to_str().unwrap();
            let before = load_module_source(name, &source, path.parent().unwrap()).unwrap();
            let after = load_module_source(name, &formatted, path.parent().unwrap()).unwrap();
            assert_eq!(before.module, after.module, "{}", path.display());
            let comments = |text: &str| {
                text.lines()
                    .filter_map(|line| {
                        let start = line.find("//").or_else(|| line.find('#'))?;
                        Some(line[start..].trim_end().to_owned())
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                comments(&source),
                comments(&formatted),
                "{}",
                path.display()
            );
        }
    }
}

fn source_files(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_none_or(|name| name != "diagnostics") {
                source_files(&path, files);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "soda")
        {
            files.push(path);
        }
    }
}

#[test]
fn grouped_datatypes_format_independently_and_keep_their_ast() {
    let source = "module M; datatype V{constructor v(Int);constructor empty();} datatype N{constructor zero();constructor succ(N);}; constructor extra():V; eval v(1);";
    let formatted = soda_parser::format_source(source).unwrap();
    assert_eq!(
        soda_parser::parse_module(source).unwrap(),
        soda_parser::parse_module(&formatted).unwrap()
    );
    assert_eq!(soda_parser::format_source(&formatted).unwrap(), formatted);
    assert!(formatted.contains("    constructor v(Int);"), "{formatted}");
    assert!(formatted.contains("}\n\ndatatype N"), "{formatted}");
}

