use soda_ast::{DomainDecl, DomainExpr, SemanticMap, Term};
use soda_domains::{Domains, Environment, Limits};

fn app(name: &str, args: Vec<Term>) -> Term {
    Term::app(name, args)
}

fn bottom() -> Term {
    app("bottom", vec![])
}

fn declaration(name: &str, expression: DomainExpr) -> DomainDecl {
    DomainDecl {
        name: name.into(),
        expression,
    }
}

fn boolean() -> DomainDecl {
    declaration(
        "B",
        DomainExpr::Flat(vec![Term::Bool(false), Term::Bool(true)]),
    )
}

fn domains() -> Domains {
    Domains::new(
        &[
            declaration("F", DomainExpr::Function("B".into(), "B".into())),
            declaration("P", DomainExpr::Product("B".into(), "B".into())),
            declaration("L", DomainExpr::Lifted("B".into())),
            boolean(),
        ],
        Limits::default(),
    )
    .unwrap()
}

fn map(name: &str, entries: Vec<(Term, Term)>) -> SemanticMap {
    SemanticMap {
        name: name.into(),
        input: "B".into(),
        output: "B".into(),
        entries,
    }
}

#[test]
fn products_lifts_and_function_spaces_are_exact_pointed_posets() {
    let registry = domains();

    for (name, cardinality) in [("B", 3), ("P", 9), ("L", 4), ("F", 11)] {
        let domain = registry.get(name).unwrap();

        assert_eq!(domain.elements().len(), cardinality);

        for x in domain.elements() {
            assert!(domain.less_equal(domain.bottom(), x).unwrap());
            assert!(domain.less_equal(x, x).unwrap());

            for y in domain.elements() {
                if domain.less_equal(x, y).unwrap() && domain.less_equal(y, x).unwrap() {
                    assert_eq!(x, y);
                }

                for z in domain.elements() {
                    if domain.less_equal(x, y).unwrap() && domain.less_equal(y, z).unwrap() {
                        assert!(domain.less_equal(x, z).unwrap());
                    }
                }
            }
        }
    }

    let lifted = registry.get("L").unwrap();

    assert!(
        lifted
            .less_equal(&bottom(), &app("lift", vec![bottom()]))
            .unwrap()
    );
    assert!(
        !lifted
            .less_equal(&app("lift", vec![bottom()]), &bottom())
            .unwrap()
    );
    assert!(
        !registry
            .get("B")
            .unwrap()
            .less_equal(&Term::Bool(false), &Term::Bool(true))
            .unwrap()
    );
}

#[test]
fn total_monotone_maps_and_nonbottom_least_fixed_points_are_checked() {
    let registry = domains();
    let b = bottom();
    let t = Term::Bool(true);
    let f = Term::Bool(false);
    let constant = registry
        .compile(&map(
            "constant",
            vec![
                (b.clone(), t.clone()),
                (f.clone(), t.clone()),
                (t.clone(), t.clone()),
            ],
        ))
        .unwrap();

    assert_eq!(registry.apply(&constant, &f).unwrap(), t);
    let fixed = registry.least_fixed_point(&constant).unwrap();

    assert_eq!(fixed.value, t);
    assert_eq!(fixed.chain, vec![b.clone(), t.clone()]);
    assert!(
        registry
            .compile(&map("partial", vec![(b.clone(), b.clone())]))
            .is_err()
    );
    assert!(
        registry
            .compile(&map(
                "duplicate",
                vec![(b.clone(), b.clone()), (b.clone(), t.clone())]
            ))
            .is_err()
    );
    assert!(
        registry
            .compile(&map(
                "nonmonotone",
                vec![(b.clone(), t.clone()), (f.clone(), f), (t, b.clone())]
            ))
            .is_err()
    );
    assert!(
        registry
            .compile(&map("outside", vec![(Term::Int(8), b)]))
            .is_err()
    );
}

