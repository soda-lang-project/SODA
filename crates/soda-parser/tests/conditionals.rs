use soda_parser::{format_source, parse_module, parse_term};

#[test]
fn both_conditional_spellings_have_the_same_ast() {
    for (surface, call) in [
        ("if true then 1 else 2", "if(true, 1, 2)"),
        (
            "if (gt(3, 2)) then add(1, 2) else 0",
            "if(gt(3, 2), add(1, 2), 0)",
        ),
        (
            "if if true then false else true then 1 else 2",
            "if(if(true, false, true), 1, 2)",
        ),
        (
            "if true then if false then 1 else 2 else if true then 3 else 4",
            "if(true, if(false, 1, 2), if(true, 3, 4))",
        ),
        (
            "f(if ?c then ?x else ?y, if(true, 1, 2))",
            "f(if(?c, ?x, ?y), if(true, 1, 2))",
        ),
    ] {
        assert_eq!(
            parse_term(surface).unwrap(),
            parse_term(call).unwrap(),
            "{surface}"
        );
    }
}

#[test]
fn conditional_errors_and_nesting_are_bounded() {
    for source in [
        "if true then 1",
        "if true else 2",
        "if true then else",
        "if then 1 else 2",
    ] {
        assert!(parse_term(source).is_err(), "{source}");
    }
    let source = format!("{}1{}", "if true then ".repeat(260), " else 0".repeat(260));
    assert!(parse_term(&source).is_err());
}

#[test]
fn inference_if_and_conditional_terms_do_not_conflict() {
    let module =
        parse_module("module X; infer r: p(?x) if q(?x) when if true then true else false;")
            .unwrap();
    assert_eq!(module.clauses.len(), 1);
    let source = "module X; rule r: f(?c) => if(?c, if(true, 1, 2), 3);";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("if ?c\n"));
    assert!(formatted.contains("then\n"));
    assert!(formatted.contains("if true then 1 else 2"));
    assert_eq!(
        parse_module(source).unwrap(),
        parse_module(&formatted).unwrap()
    );
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

