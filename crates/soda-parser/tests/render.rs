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

