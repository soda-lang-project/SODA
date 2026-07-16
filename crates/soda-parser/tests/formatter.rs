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

