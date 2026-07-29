use soda_parser::{format_source, parse_module};

#[test]
fn multiline_branches_align_and_nested_branches_have_their_own_block() {
    let source = "module X; eval if greater_than_the_allowed_limit(1) then true else retry_the_entire_operation(2);";
    let formatted = format_source(source).unwrap();
    assert_eq!(
        formatted,
        "module X;\n\neval if greater_than_the_allowed_limit(1)\n     then true\n     else retry_the_entire_operation(2);\n"
    );
    let nested = format_source("module X; eval if ?c then if ?d then 1 else 2 else 3;").unwrap();
    assert_eq!(
        nested,
        "module X;\n\neval if ?c\n     then\n         if ?d then 1 else 2\n     else 3;\n"
    );
}

#[test]
fn argument_comments_stay_with_arguments_and_comment_paragraphs_survive() {
    let source =
        "module X;\n# first paragraph\n\n# second paragraph\neval f(1, // first argument\n2);";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("# first paragraph\n\n# second paragraph"));
    assert!(
        formatted.contains("1, // first argument\n    2"),
        "{formatted}"
    );
    assert_eq!(
        parse_module(source).unwrap(),
        parse_module(&formatted).unwrap()
    );
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

