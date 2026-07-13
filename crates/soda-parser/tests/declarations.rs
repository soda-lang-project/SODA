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

