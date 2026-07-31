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

#[test]
fn commands_do_not_rewrite_strings_comments_or_identifiers() {
    let source = r#"module Safe;
        judgment j(E,E);
        notation j "$0 → $1" ascii "$0 -> $1";
        // \lambda malformed text and \llbracket
        # \bot is ignored here
        eval lambda_name;
        eval j(vdash, llbracket);
    "#;
    let parsed = parse_module_located(source).unwrap();

    assert_eq!(parsed.module.notations[0].unicode, "$0 → $1");
    assert_eq!(
        parsed.module.evaluations[0],
        parse_term("lambda_name").unwrap()
    );
    assert_eq!(
        parsed.module.evaluations[1],
        parse_term("j(vdash,llbracket)").unwrap()
    );
    // String escapes are decoded before TeX notation commands.

    let escaped = parse_module(r#"module M; judgment j(E); notation j "\\llbracket $0 \\rrbracket" ascii "[| $0 |]"; eval \llbracket 1 \rrbracket;"#).unwrap();

    assert_eq!(escaped.notations[0].unicode, r"\llbracket $0 \rrbracket");
    assert_eq!(escaped.evaluations[0], parse_term("j(1)").unwrap());
}
