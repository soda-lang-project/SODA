use crate::diagnostics::Context;
use serde_json::{Value, json};
use soda_diagnostics::{Report, Span};
use soda_parser::{
    DeclarationKind,
    lexer::{Kind, lex},
};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Read, Write},
};
use url::Url;

const MAX_MESSAGE: usize = 16 * 1024 * 1024;
const KEYWORDS: &[&str] = &[
    "module",
    "import",
    "sort",
    "datatype",
    "constructor",
    "function",
    "judgment",
    "notation",
    "rule",
    "axiom",
    "proof",
    "infer",
    "query",
    "eval",
    "induction",
    "case",
    "record",
    "monad",
    "domain",
    "map",
    "fixed",
    "when",
    "if",
    "then",
    "else",
    "let",
    "do",
    "return",
    "bind",
    "lambda",
    "with",
    "at",
    "on",
    "reverse",
    "ascii",
    "kind",
    "true",
    "false",
    "Int",
    "Bool",
    "Name",
    "Fn",
];

struct Symbol {
    name: String,
    kind: DeclarationKind,
    uri: String,
    range: Value,
    selection: Value,
    detail: String,
    parameters: Option<Vec<String>>,
}
struct Document {
    text: String,
    version: i64,
    symbols: Vec<Symbol>,
    diagnostics: Vec<Value>,
    actions: Vec<Value>,
    virtual_sources: BTreeMap<String, String>,
}

/// LSP positions count UTF-16 code units, while the compiler uses UTF-8 bytes.
fn position(text: &str, offset: usize) -> Value {
    let mut end = offset.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let prefix = &text[..end];
    let start = prefix.rfind('\n').map_or(0, |i| i + 1);
    json!({"line": prefix.bytes().filter(|b| *b == b'\n').count(),
        "character": text[start..end].encode_utf16().count()})
}
fn range(text: &str, span: Span) -> Value {
    json!({"start": position(text, span.start), "end": position(text, span.end)})
}
fn offset(text: &str, pos: &Value) -> Option<usize> {
    let line = pos["line"].as_u64()? as usize;
    let target = pos["character"].as_u64()? as usize;
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1)?.0 + 1
    };
    let mut units = 0;
    for (i, ch) in text[start..].char_indices() {
        if units == target {
            return Some(start + i);
        }
        if ch == '\n' || ch == '\r' {
            return None;
        }
        units += ch.len_utf16();
        if units > target {
            return None;
        }
    }
    (units == target).then_some(text.len())
}
fn file_path(uri: &str) -> Option<std::path::PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}
fn source_uri(name: &str) -> Option<String> {
    if name.starts_with("<soda.") && name.ends_with('>') {
        return Some(format!("soda-stdlib:/{}", &name[1..name.len() - 1]));
    }
    Url::from_file_path(std::path::absolute(name).ok()?)
        .ok()
        .map(Into::into)
}

mod analysis;
mod features;
mod protocol;
use analysis::analyze;
use features::feature;
pub(crate) use protocol::serve;
#[cfg(test)]
use protocol::{read_message, send};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_positions_roundtrip() {
        let text = "module 𝒜;\r\nrule λx: f => 1;\n";
        for (byte, _) in text.char_indices().filter(|(_, c)| *c != '\n') {
            assert_eq!(offset(text, &position(text, byte)), Some(byte));
        }
        assert_eq!(offset("𝒜", &json!({"line":0,"character":1})), None);
    }
    #[test]
    fn framing_is_byte_counted_and_bounded() {
        let mut bytes = Vec::new();
        send(&mut bytes, json!({"method":"λ"})).unwrap();
        assert_eq!(
            read_message(&mut io::Cursor::new(bytes)).unwrap().unwrap()["method"],
            "λ"
        );
        assert!(read_message(&mut io::Cursor::new(b"Content-Length: 999999999\r\n\r\n")).is_err());
    }
    #[test]
    fn analyzer_uses_compiler_sorts_and_navigation() {
        let uri = source_uri("editor-test.soda").unwrap();
        let doc = analyze(
            &uri,
            "module Test;\nsort E;\nfunction f(Int): Int;\nrule bad: f(true) => 1;\n".into(),
            1,
        );
        assert!(!doc.diagnostics.is_empty());
        assert!(
            doc.diagnostics[0]["message"]
                .as_str()
                .unwrap()
                .contains("sort")
        );
        assert!(doc.symbols.iter().any(|s| s.name == "f"));
        let definitions = feature(
            "textDocument/definition",
            &json!({"position":{"line":3,"character":11}}),
            &doc,
            &uri,
        )
        .unwrap();
        assert!(!definitions.as_array().unwrap().is_empty());
    }

    #[test]
    fn incomplete_edits_keep_current_declarations_and_signature_help() {
        let uri = source_uri("unfinished.soda").unwrap();
        let text = "module Test;\nfunction f(Int, Int): Int;\neval f(add(1, 2), ";
        let doc = analyze(&uri, text.into(), 2);
        assert!(!doc.diagnostics.is_empty());
        assert!(doc.symbols.iter().any(|s| s.name == "f"));
        let help = feature(
            "textDocument/signatureHelp",
            &json!({"position":position(text, text.len())}),
            &doc,
            &uri,
        )
        .unwrap();
        assert_eq!(help["activeParameter"], 1);
        assert!(
            help["signatures"][0]["label"]
                .as_str()
                .unwrap()
                .contains("function f")
        );
    }

    #[test]
    fn builtin_import_navigation_has_real_virtual_source_content() {
        let uri = source_uri("import-test.soda").unwrap();
        let doc = analyze(&uri, "module Test; import soda.data.nat;".into(), 1);
        assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
        assert!(
            doc.symbols
                .iter()
                .any(|s| s.uri.starts_with("soda-stdlib:"))
        );
        assert!(
            doc.virtual_sources
                .values()
                .any(|text| text.contains("constructor"))
        );
    }

    #[test]
    fn references_skip_comments_metavariables_and_object_names() {
        let uri = source_uri("references.soda").unwrap();
        let text = "module Test;\nfunction f(Int): Int;\nrule r: f(?f) => ?f; // f\neval f(1);";
        let doc = analyze(&uri, text.into(), 1);
        let refs = feature(
            "textDocument/references",
            &json!({"position":{"line":3,"character":6},"context":{"includeDeclaration":false}}),
            &doc,
            &uri,
        )
        .unwrap();
        assert_eq!(refs.as_array().unwrap().len(), 2);
    }

    #[test]
    fn quick_fix_applies_to_the_current_buffer_and_clears_the_error() {
        let uri = source_uri("quickfix.soda").unwrap();
        let text = "module Test;\nrule r: f => 1";
        let doc = analyze(&uri, text.into(), 1);
        let edit = &doc.actions[0]["edit"]["changes"][&uri][0];
        let start = offset(text, &edit["range"]["start"]).unwrap();
        let end = offset(text, &edit["range"]["end"]).unwrap();
        let mut fixed = text.to_owned();
        fixed.replace_range(start..end, edit["newText"].as_str().unwrap());
        assert!(analyze(&uri, fixed, 2).diagnostics.is_empty());
    }
}
