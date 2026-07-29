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

