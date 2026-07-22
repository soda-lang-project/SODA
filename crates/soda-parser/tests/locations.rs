use soda_ast::Term;
use soda_parser::{parse_module, parse_module_located, parse_term_located_in};

#[test]
fn exact_subterm_and_declaration_ranges_survive_comments_crlf_and_unicode() {
    let text = "module Μ;\r\n# true is a comment\r\nfunction f(Int): Int;\r\nrule f: f(?n) => add(?n, true);\r\n";
    let parsed = parse_module_located(text).unwrap();

    assert_eq!(parsed.module, parse_module(text).unwrap());
    let rule = parsed
        .locations
        .declarations
        .iter()
        .find(|site| site.kind == soda_parser::DeclarationKind::Rule)
        .unwrap();

    assert_eq!(&text[rule.name_span.start..rule.name_span.end], "f");
    let span = parsed
        .locations
        .term(&Term::Bool(true), Some(rule.span))
        .unwrap();

    assert_eq!(&text[span.start..span.end], "true");
    assert_eq!(span.start, text.rfind("true").unwrap());
}

