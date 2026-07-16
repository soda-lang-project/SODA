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

