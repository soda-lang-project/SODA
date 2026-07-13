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

