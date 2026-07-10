use soda_ast::{DomainExpr, Term};
use soda_parser::{parse_module, parse_term};

#[test]
fn domains_maps_and_fixed_points_preserve_their_structure() {
    let module = parse_module(
        r#"
      module Domains;
      domain D = flat(a,b);
      domain P = product(D,D);
      domain F = functions(D,D);
      domain L = lift(D);
      map f: D -> D { ⊥ => a; a => a; b => a; }
      fixed f;
    "#,
    )
    .unwrap();

    assert_eq!(
        module.domains[0].expression,
        DomainExpr::Flat(vec![Term::app("a", vec![]), Term::app("b", vec![])])
    );
    assert_eq!(
        module.domains[1].expression,
        DomainExpr::Product("D".into(), "D".into())
    );
    assert_eq!(
        module.domains[2].expression,
        DomainExpr::Function("D".into(), "D".into())
    );
    assert_eq!(module.domains[3].expression, DomainExpr::Lifted("D".into()));
    assert_eq!(module.semantic_maps[0].entries.len(), 3);
    assert_eq!(
        module.semantic_maps[0].entries[0].0,
        parse_term("bottom").unwrap()
    );
    assert_eq!(module.fixed_points, vec!["f"]);
}

#[test]
fn induction_cases_use_the_same_rewrite_certificates_as_equational_proofs() {
    let module = parse_module(
        r#"
      module Nat;
      datatype Nat;
      constructor zero(): Nat;
      constructor succ(Nat): Nat;
      function plus(Nat,Nat): Nat;
      induction right_zero on ?n: Nat: plus(?n,zero) = ?n {
        case zero { plus_zero; }
        case succ(k) { plus_succ with {n=?k}; ih_k at [0]; }
      }
    "#,
    )
    .unwrap();
    let proof = &module.inductions[0];

    assert_eq!(proof.name, "right_zero");
    assert_eq!(proof.variable, "n");
    assert_eq!(proof.sort, "Nat");
    assert_eq!(proof.cases[0].constructor, "zero");
    assert_eq!(proof.cases[1].arguments, vec!["k"]);
    assert_eq!(proof.cases[1].steps[1].axiom, "ih_k");
    assert_eq!(proof.cases[1].steps[1].path, vec![0]);
}

#[test]
fn malformed_domain_and_induction_syntax_is_rejected() {
    for source in [
        "domain D = unknown();",
        "domain D = product(A);",
        "domain D = flat(a,);",
        "domain D = lift(A,B);",
        "map f: D D {}",
        "map f: D -> D { a = b; }",
        "map f: D -> D { a => b }",
        "fixed;",
        "induction p ?n: N: a = b {}",
        "induction p on n: N: a = b {}",
        "induction p on ?n: N: a = b { nope }",
        "induction p on ?n: N: a = b { case c(x,) {} }",
        "induction p on ?n: N: a = b { case c { missing } }",
        "induction p on ?n: N: a = b {} proof p: a = b {}",
    ] {
        assert!(
            parse_module(&format!("module Bad; {source}")).is_err(),
            "accepted {source}"
        );
    }

    let deeply_nested = format!(
        "module Deep; function f(): {}N{};",
        "Fn(".repeat(300),
        ")".repeat(300)
    );

    assert!(
        parse_module(&deeply_nested)
            .unwrap_err()
            .message
            .contains("nesting")
    );
}

#[test]
fn named_premises_and_side_conditions_preserve_labels_without_changing_judgments() {
    let module=parse_module("module M; infer r: p(?x) if first: q(?x), q(?x) when fresh(@y, ?x), true; infer fact: p(a) when true; infer plain: p(a) if q(a);").unwrap();

    assert_eq!(module.clauses.len(), 3);
    assert_eq!(module.clause_details.len(), 2);
    assert_eq!(module.clause_details[0].clause, "r");
    assert_eq!(
        module.clause_details[0].premise_labels,
        vec![Some("first".into()), None]
    );
    assert_eq!(module.clause_details[0].conditions.len(), 2);
    assert!(module.clause_details[1].premise_labels.is_empty());

    for source in [
        "infer r: p when;",
        "infer r: p if label:;",
        "infer r: p when true,;",
    ] {
        assert!(parse_module(&format!("module M; {source}")).is_err());
    }
}

#[test]
fn computed_bindings_preserve_interleaved_and_trailing_order() {
    let module = parse_module(
        r#"
        module Computed;
        infer add: result(?r) if first(?x), let ?y = add(?x, 1), second(?y)
            let ?r = mul(?y, 2);
        infer direct: result(?r) let ?r = add(2, 3);
    "#,
    )
    .unwrap();
    assert_eq!(module.clauses[0].premises.len(), 2);
    assert_eq!(module.clause_details[0].bindings.len(), 2);
    assert_eq!(module.clause_details[0].bindings[0].before_premise, 1);
    assert_eq!(module.clause_details[0].bindings[1].before_premise, 2);
    assert_eq!(module.clause_details[1].bindings[0].before_premise, 0);
}
