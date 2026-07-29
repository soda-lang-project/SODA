use soda_ast::{JudgmentDecl, JudgmentKind, Module, NotationDecl, Term};
use soda_parser::{NotationStyle, format_term_in, parse_module, parse_term, parse_term_in};

fn notation_module() -> Module {
    parse_module(
        r#"
        module Notation;
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
    "#,
    )
    .unwrap()
}

#[test]
fn notation_styles_round_trip_nested_judgments_constructors_and_binders() {
    let module = notation_module();
    let sources = [
        "typing(Γ, bind(x: Expr, step(@x, @x)), Nat)",
        "denote(step(a,b), ρ)",
        "hoare(satisfies(m, true), skip, satisfies(m, false))",
        "step(step(a, b), step(c, d))",
        "evals(wrap(typing(Γ, @x, Nat)), 1)",
        "satisfies(denote(a, ρ), true)",
        "bind(x: Expr, denote(@x, ρ))",
    ];

    for source in sources {
        let term = parse_term(source).unwrap();

        for style in [
            NotationStyle::Canonical,
            NotationStyle::Unicode,
            NotationStyle::Ascii,
        ] {
            let rendered = format_term_in(&module, &term, style);

            assert_eq!(
                parse_term_in(&module, &rendered).unwrap(),
                term,
                "{style:?}: {rendered}"
            );
        }
    }

    let term = parse_term("step(a,step(b,c))").unwrap();

    assert_eq!(
        format_term_in(&module, &term, NotationStyle::Unicode),
        "a → (b → c)"
    );
    assert_eq!(
        format_term_in(&module, &term, NotationStyle::Ascii),
        "a -> (b -> c)"
    );
}

#[test]
fn templates_support_zero_arguments_and_multidigit_reordered_holes() {
    let mut module =
        parse_module("module M; judgment unit(); notation unit \"◇\" ascii \"<>\";").unwrap();
    let order = [11, 1, 10, 0, 2, 3, 4, 5, 6, 7, 8, 9];
    let holes = order
        .iter()
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>()
        .join(" : ");
    module.judgments.push(JudgmentDecl {
        name: "many".into(),
        arguments: vec!["A".into(); 12],
        kind: JudgmentKind::Relation,
    });
    module.notations.push(NotationDecl {
        judgment: "many".into(),
        unicode: format!("⟨ {holes} ⟩"),
        ascii: format!("[ {holes} ]"),
    });
    for term in [
        Term::app("unit", vec![]),
        Term::app("many", (0..12).map(Term::Int).collect()),
    ] {
        for style in [NotationStyle::Unicode, NotationStyle::Ascii] {
            let text = format_term_in(&module, &term, style);

            assert_eq!(parse_term_in(&module, &text).unwrap(), term, "{text}");
        }
    }

    assert!(
        format_term_in(
            &module,
            &Term::app("many", (0..12).map(Term::Int).collect()),
            NotationStyle::Unicode
        )
        .starts_with("⟨ 11 : 1 : 10 : 0")
    );
}

#[test]
fn invalid_programmatic_metadata_falls_back_without_panicking() {
    let term = Term::app("j", vec![Term::Int(1)]);

    for template in [
        "",
        "$",
        "$999999999999999999999999999",
        "$1 →",
        "$0 $0",
        "$0",
        "\"",
        "$0 # comment",
    ] {
        let module = Module {
            judgments: vec![JudgmentDecl {
                name: "j".into(),
                arguments: vec!["A".into()],
                kind: JudgmentKind::Relation,
            }],
            notations: vec![NotationDecl {
                judgment: "j".into(),
                unicode: template.into(),
                ascii: template.into(),
            }],
            ..Module::default()
        };

        assert_eq!(
            format_term_in(&module, &term, NotationStyle::Unicode),
            term.to_string()
        );
    }

    let mut missing = Module::default();
    missing.notations.push(NotationDecl {
        judgment: "j".into(),
        unicode: "$0 →".into(),
        ascii: "$0 ->".into(),
    });

    assert_eq!(
        format_term_in(&missing, &term, NotationStyle::Ascii),
        term.to_string()
    );
    let mut ambiguous = notation_module();
    ambiguous.notations.push(NotationDecl {
        judgment: "evals".into(),
        unicode: "$0 → $1".into(),
        ascii: "$0 -> $1".into(),
    });

    assert_eq!(
        format_term_in(&ambiguous, &term, NotationStyle::Ascii),
        term.to_string()
    );
    assert_eq!(
        format_term_in(&notation_module(), &term, NotationStyle::Canonical),
        term.to_string()
    );
}

#[test]
fn literal_runs_preserve_lexer_boundaries_between_punctuation() {
    let module = parse_module(
        "module M; judgment j(A,A); notation j \"$0 / / = > $1\" ascii \"$0 / / = > $1\";",
    )
    .unwrap();
    let term = Term::app("j", vec![Term::Int(1), Term::Int(-1)]);
    let output = format_term_in(&module, &term, NotationStyle::Ascii);

    assert_eq!(parse_term_in(&module, &output).unwrap(), term, "{output}");
}
