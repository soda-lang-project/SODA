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

#[test]
fn mathematical_colons_are_spaced_and_binder_colons_stay_attached() {
    let source = "module X; sort C; sort E; sort T; constructor empty(): C; constructor e(): E; constructor t(): T; judgment typing(C, E, T) kind typing; notation typing \"$0 ⊢ $1 : $2\" ascii \"$0 |- $1 : $2\"; infer fact: empty ⊢ e:t; query empty |- e:t; eval lambda x:Int.@x;";
    let formatted = format_source(source).unwrap();
    assert!(
        formatted.contains("infer fact: empty ⊢ e : t;"),
        "{formatted}"
    );
    assert!(formatted.contains("query empty |- e : t;"), "{formatted}");
    assert!(formatted.contains("lambda x: Int. @x"));
    assert_eq!(
        parse_module(source).unwrap(),
        parse_module(&formatted).unwrap()
    );
    assert_eq!(formatted, format_source(&formatted).unwrap());
}

#[test]
fn distinct_subjects_get_separate_families_even_with_similar_rule_names() {
    let source = "module X; rule valid_zero: a(0) => true; rule valid_one: a(1) => true; rule valid_other: b(0) => true;";
    let formatted = format_source(source).unwrap();
    assert!(formatted.contains("valid_zero: a(0) => true;\nrule valid_one"));
    assert!(formatted.contains("valid_one: a(1) => true;\n\nrule valid_other"));
}

#[test]
fn generated_nested_expressions_preserve_ast_width_and_idempotence() {
    fn term(seed: &mut u64, depth: usize) -> soda_ast::Term {
        use soda_ast::Term;
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let choice = *seed >> 32;
        if depth == 0 {
            return if choice.is_multiple_of(2) {
                Term::Int((choice % 100) as i64)
            } else {
                Term::Bool(choice.is_multiple_of(3))
            };
        }
        match choice % 5 {
            0 => Term::app(
                "if",
                vec![
                    term(seed, depth - 1),
                    term(seed, depth - 1),
                    term(seed, depth - 1),
                ],
            ),
            1 => Term::app("pair", vec![term(seed, depth - 1), term(seed, depth - 1)]),
            2 => Term::app("add", vec![term(seed, depth - 1), term(seed, depth - 1)]),
            3 => Term::Binder {
                parameter: "x".into(),
                sort: "Int".into(),
                body: Box::new(term(seed, depth - 1)),
            },
            _ => Term::app("node", vec![term(seed, depth - 1)]),
        }
    }
    let mut seed = 17;
    for case in 0..256 {
        let source = format!("module X; eval {};", term(&mut seed, 4));
        let formatted =
            format_source(&source).unwrap_or_else(|error| panic!("case {case}: {error}"));
        assert_eq!(
            parse_module(&source).unwrap(),
            parse_module(&formatted).unwrap(),
            "case {case}"
        );
        assert_eq!(formatted, format_source(&formatted).unwrap(), "case {case}");
        assert!(
            formatted.lines().all(|line| line.chars().count() <= 100),
            "case {case}:\n{formatted}"
        );
    }
}
