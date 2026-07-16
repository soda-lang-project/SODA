use soda_ast::{JudgmentKind, SignatureKind, Term};
use soda_parser::{parse_module, parse_term, parse_term_in};

#[test]
fn declarations_preserve_sorts_kinds_and_function_sorts() {
    let module = parse_module("module Typed; sort Expr; datatype Nat; constructor zero(): Nat; function f(Nat, Fn(Nat, Nat)): Nat; judgment step(Expr,Expr) kind transition; judgment valid(Nat);").unwrap();

    assert!(!module.sorts[0].inductive);
    assert!(module.sorts[1].inductive);
    assert_eq!(module.signatures[0].kind, SignatureKind::Constructor);
    assert_eq!(module.signatures[1].kind, SignatureKind::Function);
    assert_eq!(module.signatures[1].arguments[1], "Fn(Nat,Nat)");
    assert_eq!(module.judgments[0].kind, JudgmentKind::Transition);
    assert_eq!(module.judgments[1].kind, JudgmentKind::Relation);
}

#[test]
fn binding_forms_share_one_ast_and_round_trip() {
    let expected = Term::Binder {
        parameter: "x".into(),
        sort: "Expr".into(),
        body: Box::new(Term::Name("x".into())),
    };

    for source in ["bind(x: Expr, @x)", "lambda x: Expr. @x", "λx: Expr. @x"] {
        assert_eq!(parse_term(source).unwrap(), expected);
    }

    assert_eq!(parse_term(&expected.to_string()).unwrap(), expected);
    assert_eq!(parse_term("Γ(@ρ)").unwrap().to_string(), "Γ(@ρ)");
}

#[test]
fn mathematical_notation_and_ascii_lower_to_identical_judgments() {
    let module = parse_module(
        r#"
        module Math;
        judgment typing(Context, Expr, Type) kind typing;
        notation typing "$0 ⊢ $1 : $2" ascii "$0 |- $1 : $2";
        judgment denote(Expr, Context) kind denotation;
        notation denote "⟦ $0 ⟧ $1" ascii "[| $0 |] $1";
        judgment hoare(Assertion, Expr, Assertion) kind hoare;
        notation hoare "{ $0 } $1 { $2 }" ascii "{| $0 |} $1 {| $2 |}";
        judgment step(Expr, Expr) kind transition;
        notation step "$0 → $1" ascii "$0 -> $1";
        judgment evals(Expr, Value) kind evaluation;
        notation evals "$0 ⇓ $1" ascii "$0 ~> $1";
        judgment satisfies(Model, Assertion) kind satisfaction;
        notation satisfies "$0 ⊨ $1" ascii "$0 |= $1";
        infer variable: Γ ⊢ @x : Nat;
        query Γ |- @x : Nat;
    "#,
    )
    .unwrap();

    assert_eq!(module.clauses[0].conclusion, module.queries[0]);
    for (unicode, ascii, plain) in [
        ("Γ ⊢ @x : Nat", "Γ |- @x : Nat", "typing(Γ, @x, Nat)"),
        (
            "⟦ plus(1, 2) ⟧ ρ",
            "[| plus(1, 2) |] ρ",
            "denote(plus(1, 2), ρ)",
        ),
        (
            "{ true } skip { true }",
            "{| true |} skip {| true |}",
            "hoare(true, skip, true)",
        ),
        ("a → b", "a -> b", "step(a,b)"),
        ("a ⇓ 1", "a ~> 1", "evals(a,1)"),
        ("m ⊨ true", "m |= true", "satisfies(m,true)"),
    ] {
        let expected = parse_term(plain).unwrap();

        assert_eq!(parse_term_in(&module, unicode).unwrap(), expected);
        assert_eq!(parse_term_in(&module, ascii).unwrap(), expected);
    }

    assert_eq!(
        parse_term_in(&module, "(a → b) → c").unwrap(),
        parse_term("step(step(a,b),c)").unwrap()
    );
}

#[test]
fn malformed_and_ambiguous_notation_is_rejected() {
    for declaration in [
        "notation missing \"$0 → $1\" ascii \"$0 -> $1\";",
        "judgment j(A,A); notation j \"$0 $1\" ascii \"$0 -> $1\";",
        "judgment j(A,A); notation j \"$0 → $0\" ascii \"$0 -> $1\";",
        "judgment j(A,A); notation j \"$0 → $2\" ascii \"$0 -> $1\";",
        "judgment j(A,A); notation j \"$0 →\" ascii \"$0 -> $1\";",
        "judgment j(A,A); notation j \"$0 → $1\" ascii \"$0 -> $1\"; judgment k(A,A); notation k \"$0 → $1\" ascii \"$0 ~> $1\";",
        "judgment j(A); notation j \"$0\" ascii \"$0\";",
        "judgment j(A); notation j \"$-1 →\" ascii \"$0 ->\";",
        "judgment j(A); notation j \"$0 →\";",
        "judgment j(A) kind unsupported;",
        "notation j \"unterminated;",
    ] {
        assert!(
            parse_module(&format!("module Bad; {declaration}")).is_err(),
            "accepted {declaration}"
        );
    }
}

#[test]
fn notation_nesting_is_bounded_and_requires_grouping() {
    let module =
        parse_module("module M; judgment j(A,A); notation j \"$0 → $1\" ascii \"$0 -> $1\";")
            .unwrap();

    assert!(parse_term_in(&module, "a → b → c").is_err());
    assert!(parse_term_in(&module, "a →").is_err());
    assert!(
        parse_term_in(&module, &format!("{}x{}", "(".repeat(300), ")".repeat(300)))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}

#[test]
fn binder_headers_preserve_existing_bind_and_lambda_constructors() {
    let module = parse_module(include_str!("../../../examples/imperative.soda")).unwrap();

    assert_eq!(module.name, "Imperative");
    assert_eq!(
        module.rules[0].lhs.to_string(),
        "lookup(bind(?key, ?value, ?rest), ?key)"
    );
    for source in [
        "bind(?key, ?value, ?rest)",
        "bind(x,y,z)",
        "bind",
        "bind()",
        "lambda(@x)",
        "lambda",
        "lambda()",
    ] {
        let term = parse_term(source).unwrap();

        assert!(matches!(term, Term::App(..)), "{source}");
        assert_eq!(parse_term(&term.to_string()).unwrap(), term);
    }

    for source in ["bind(x: Expr, @x)", "lambda x: Expr. @x", "λx: Expr. @x"] {
        assert!(
            matches!(parse_term(source).unwrap(), Term::Binder { .. }),
            "{source}"
        );
    }
}

#[test]
fn shared_notation_prefixes_select_by_later_punctuation_and_round_trip() {
    use soda_parser::{NotationStyle, format_term_in, parse_term_located_in};

    let declarations = [
        r#"judgment typing(C,E,T); notation typing "$0 ⊢ $1 : $2" ascii "$0 |- $1 : $2";"#,
        r#"judgment evaluates(C,E,V); notation evaluates "$0 ⊢ $1 ⇓ $2" ascii "$0 |- $1 ~> $2";"#,
        r#"judgment left(E,V); notation left "⟦ $1 ⟧ : $0" ascii "[[ $1 ]] : $0";"#,
        r#"judgment right(E,V); notation right "⟦ $0 ⟧ ⇓ $1" ascii "[[ $0 ]] ~> $1";"#,
    ];

    for reversed in [false, true] {
        let mut declarations = declarations;

        if reversed {
            declarations.reverse();
        }

        let module = parse_module(&format!("module Shared; {}", declarations.join("\n"))).unwrap();

        for source in [
            "typing(g, e, t)",
            "evaluates(env, e, v)",
            "left(e, v)",
            "right(e, v)",
            "typing(g, evaluates(env, e, v), t)",
        ] {
            let expected = parse_term(source).unwrap();

            for style in [NotationStyle::Unicode, NotationStyle::Ascii] {
                let printed = format_term_in(&module, &expected, style);
                let parsed = parse_term_located_in(&module, &printed).unwrap();

                assert_eq!(parsed.term, expected, "{printed}");
                let span = parsed.locations.term(&expected, None).unwrap();

                assert_eq!(&printed[span.start..span.end], printed);
            }
        }

        for invalid in ["g ⊢ e", "g ⊢ e = t", "g |- e ~>", "⟦ e ⟧ = v"] {
            assert!(parse_term_in(&module, invalid).is_err(), "{invalid}");
        }
    }
}

#[test]
fn shared_notation_rejects_identical_skeletons_prefixes_and_hole_literal_overlap() {
    for suffix in [
        r#"judgment k(A,A,A); notation k "$2 ⊢ $0 : $1" ascii "$2 |- $0 : $1";"#,
        r#"judgment k(A,A); notation k "$0 ⊢ $1" ascii "$0 |- $1";"#,
        r#"judgment k(A,A,A); notation k "$0 ⊢ $1 : $2 !" ascii "$0 |- $1 : $2 !";"#,
        r#"judgment k(A,A,A); notation k "$0 ⊢ $1 : ⟦ $2 ⟧" ascii "$0 |- $1 : [[ $2 ]]";"#,
    ] {
        let source = format!(
            r#"module Bad; judgment j(A,A,A);
            notation j "$0 ⊢ $1 : $2" ascii "$0 |- $1 : $2"; {suffix}"#
        );

        assert!(
            parse_module(&source)
                .unwrap_err()
                .message
                .contains("ambiguous")
        );
    }
}

#[test]
fn grouped_and_standalone_datatypes_have_identical_core_declarations() {
    let standalone = parse_module("module M; datatype Value; constructor values(Int, Fn(Int, Value)): Value; constructor poison(Int): Value; datatype Nat; constructor zero(): Nat; constructor succ(Nat): Nat; constructor extra(): Value;").unwrap();
    for grouped in [
        "module M; datatype Value { constructor values(Int, Int -> Value); constructor poison(Int); } datatype Nat { constructor zero(); constructor succ(Nat); } constructor extra(): Value;",
        "module M; datatype Value { constructor values(Int, Fn(Int, Value)): Value; constructor poison(Int): Value; }; datatype Nat { constructor zero(): Nat; constructor succ(Nat): Nat; }; constructor extra(): Value;",
    ] {
        assert_eq!(parse_module(grouped).unwrap(), standalone);
    }
    assert_eq!(
        parse_module("module M; datatype Empty {}").unwrap(),
        parse_module("module M; datatype Empty;").unwrap()
    );
}

#[test]
fn malformed_datatype_groups_explain_the_expected_syntax() {
    for source in [
        "datatype V { function f(): V; }",
        "datatype V { constructor x();",
        "datatype V { constructor x() }",
        "datatype V { datatype W; }",
        "sort V { constructor x(); }",
    ] {
        assert!(
            parse_module(&format!("module M; {source}")).is_err(),
            "accepted {source}"
        );
    }
    let error = parse_module("module M; datatype V { constructor x(): Int; }").unwrap_err();
    assert!(
        error.message.contains("must return enclosing datatype 'V'"),
        "{error}"
    );
}

#[test]
fn typed_function_blocks_and_compact_definitions_lower_to_ordinary_rules() {
    let old = parse_module(
        r#"module M;
        datatype Nat; constructor zero(): Nat; constructor succ(Nat): Nat;
        function add(Nat, Nat): Nat;
        rule add_zero: add(zero, ?n) => ?n;
        rule add_succ: add(succ(?m), ?n) => succ(add(?m, ?n));
        function positive(Int): Bool;
        rule positive: positive(?n) => gt(?n, 0) when ge(?n, 0);
        function apply_f(Fn(Int,Int), Int): Int;
        rule apply_f: apply_f(?f, ?x) => apply(?f, ?x);
        function constant(): Int; rule constant: constant => 7;
    "#,
    )
    .unwrap();
    let grouped = parse_module(
        r#"module M;
        datatype Nat { constructor zero(); constructor succ(Nat); }
        function add(Nat, Nat): Nat {
            case add_zero: (zero, ?n) => ?n;
            case add_succ: (succ(?m), ?n) => succ(add(?m, ?n));
        }
        function positive(?n: Int): Bool => gt(?n, 0) when ge(?n, 0);
        function apply_f(?f: Int -> Int, ?x: Int): Int => apply(?f, ?x);
        function constant(): Int => 7;
    "#,
    )
    .unwrap();
    assert_eq!(old, grouped);
    assert_eq!(
        parse_module("module M; function f(Int): Int {}").unwrap(),
        parse_module("module M; function f(Int): Int;").unwrap()
    );
}

#[test]
fn function_blocks_mix_with_standalone_rules_and_keep_rule_order() {
    let module = parse_module(
        r#"module M;
        function f(Int): Int { case first: (?x) => 1; case second: (0) => 2; };
        rule third: f(1) => 3;
        function g(): Int { case fourth: () => f(0); }
    "#,
    )
    .unwrap();
    assert_eq!(
        module
            .rules
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "second", "third", "fourth"]
    );
    assert_eq!(module.rules[3].lhs, parse_term("g").unwrap());
}

#[test]
fn invalid_function_headers_and_cases_are_rejected() {
    for source in [
        "function f(Int): Int => 1;",
        "function f(?x: Int, Int): Int => ?x;",
        "function f(?x: Int, ?x: Int): Int => ?x;",
        "function f(?x: Int): Int;",
        "function f(Int): Int { case wrong: () => 1; }",
        "function f(): Int { case wrong: (1) => 1; }",
        "function f(Int): Int { case bad: (?x) => ?x }",
        "function f(Int): Int { rule bad: f(?x) => ?x; }",
        "function f(Int): Int { case a: (?x) => ?x; case a: (?x) => ?x; }",
        "rule f: f(?x) => ?x; function f(?x: Int): Int => ?x;",
        "function f(Int): Int { case bad: (?x) => ?x;",
    ] {
        assert!(
            parse_module(&format!("module Bad; {source}")).is_err(),
            "accepted {source}"
        );
    }
    let error =
        parse_module("module Bad; function f(Int): Int { case wrong: () => 1; }").unwrap_err();
    assert!(
        error
            .message
            .contains("expects 1 argument patterns, found 0"),
        "{error}"
    );
}
