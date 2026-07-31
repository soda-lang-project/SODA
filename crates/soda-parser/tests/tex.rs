use soda_ast::Term;
use soda_parser::{parse_module, parse_module_located, parse_term, parse_term_located_in};

#[test]
fn tex_commands_are_aliases_for_declared_notation_and_keep_source_spans() {
    let module = parse_module(
        r#"module Math;
        judgment denote(E,C); notation denote "⟦ $0 ⟧ $1" ascii "[| $0 |] $1";
        judgment typing(C,E,T); notation typing "$0 ⊢ $1 : $2" ascii "$0 |- $1 : $2";
        judgment step(E,E); notation step "$0 → $1" ascii "$0 -> $1";
        judgment evals(E,V); notation evals "$0 ⇓ $1" ascii "$0 ~> $1";
        judgment holds(M,P); notation holds "$0 ⊨ $1" ascii "$0 |= $1";
        "#,
    )
    .unwrap();

    for (source, canonical) in [
        (r"\llbracket f(false) \rrbracket ρ", "denote(f(false),ρ)"),
        (r"Γ \vdash @x : Nat", "typing(Γ,@x,Nat)"),
        (r"a \rightarrow b", "step(a,b)"),
        (r"a \to b", "step(a,b)"),
        (r"a \Downarrow 1", "evals(a,1)"),
        (r"m \models true", "holds(m,true)"),
        (r"m \vDash true", "holds(m,true)"),
    ] {
        let parsed = parse_term_located_in(&module, source).unwrap();

        assert_eq!(parsed.term, parse_term(canonical).unwrap(), "{source}");
        let span = parsed.locations.term(&parsed.term, None).unwrap();

        assert_eq!(&source[span.start..span.end], source);
    }

    let source = r"\llbracket f(false) \rrbracket ρ";
    let parsed = parse_term_located_in(&module, source).unwrap();
    let span = parsed.locations.term(&Term::Bool(false), None).unwrap();

    assert_eq!(&source[span.start..span.end], "false");
    assert!(parse_term(r"a \rightarrow b").is_err());
}

#[test]
fn tex_lambda_and_bottom_use_existing_base_terms() {
    assert_eq!(
        parse_term(r"\lambda x: Expr. @x").unwrap(),
        parse_term("bind(x: Expr, @x)").unwrap()
    );
    assert_eq!(parse_term(r"\bot").unwrap(), parse_term("bottom").unwrap());

    for source in [
        r"\lambdax: Expr. @x",
        r"\bottom",
        r"\unknown",
        r"\Lambda x: Expr. @x",
    ] {
        assert!(parse_term(source).is_err(), "{source}");
    }
}

