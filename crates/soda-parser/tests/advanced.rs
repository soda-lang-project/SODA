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

