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

#[test]
fn function_elements_apply_and_preserve_all_finite_chain_suprema() {
    let registry = domains();
    let base = registry.get("B").unwrap();

    for function in registry.get("F").unwrap().elements() {
        for x in base.elements() {
            let value = registry.apply_element("F", function, x).unwrap();

            assert!(base.index(&value).is_some());

            for y in base.elements() {
                if base.less_equal(x, y).unwrap() {
                    let upper = registry.apply_element("F", function, y).unwrap();

                    assert!(base.less_equal(&value, &upper).unwrap());
                }
            }
        }
    }

    assert_eq!(
        registry
            .apply_element("F", &bottom(), &Term::Bool(true))
            .unwrap(),
        bottom()
    );
    assert!(registry.apply_element("B", &bottom(), &bottom()).is_err());
    assert!(
        registry
            .apply_element(
                "F",
                &app(
                    "table",
                    vec![Term::Bool(true), Term::Bool(false), Term::Bool(true)]
                ),
                &bottom()
            )
            .is_err()
    );
    assert!(
        registry
            .apply_element("F", &bottom(), &Term::Int(4))
            .is_err()
    );
}

#[test]
fn functions_cannot_be_transplanted_or_retargeted() {
    let registry = domains();
    let function = registry
        .compile(&map(
            "identity",
            vec![
                (bottom(), bottom()),
                (Term::Bool(false), Term::Bool(false)),
                (Term::Bool(true), Term::Bool(true)),
            ],
        ))
        .unwrap();
    let swapped = Domains::new(
        &[declaration(
            "B",
            DomainExpr::Flat(vec![Term::Bool(true), Term::Bool(false)]),
        )],
        Limits::default(),
    )
    .unwrap();

    assert!(swapped.apply(&function, &bottom()).is_err());
    assert!(swapped.least_fixed_point(&function).is_err());
    let mut retargeted = function.clone();
    retargeted.output = "L".into();

    assert!(registry.apply(&retargeted, &bottom()).is_err());
}

#[test]
fn environments_shadow_names_and_check_domain_identity_recursively() {
    let registry = domains();
    let mut environment = Environment::default();
    environment
        .bind(&registry, "x", "B", &Term::Bool(false))
        .unwrap();
    environment
        .bind(&registry, "x", "B", &Term::Bool(true))
        .unwrap();

    assert_eq!(environment.lookup("x"), Some(("B", &Term::Bool(true))));
    let x = Term::Name("x".into());

    assert_eq!(
        environment
            .interpret(&registry, "P", &app("pair", vec![x.clone(), bottom()]))
            .unwrap(),
        app("pair", vec![Term::Bool(true), bottom()])
    );
    assert!(environment.interpret(&registry, "L", &x).is_err());
    assert!(
        environment
            .bind(&registry, "bad", "B", &Term::Int(0))
            .is_err()
    );
    assert!(environment.bind(&registry, "", "B", &bottom()).is_err());
    assert!(
        environment
            .interpret(&registry, "B", &Term::Name("unknown".into()))
            .is_err()
    );
    let changed = Domains::new(
        &[declaration(
            "B",
            DomainExpr::Flat(vec![Term::Bool(true), Term::Bool(false)]),
        )],
        Limits::default(),
    )
    .unwrap();

    assert!(environment.interpret(&changed, "B", &x).is_err());
}

#[test]
fn bottom_aliases_normalize_in_nested_products_and_tables() {
    let registry = Domains::new(
        &[
            boolean(),
            declaration("P", DomainExpr::Product("B".into(), "B".into())),
            declaration("N", DomainExpr::Product("P".into(), "P".into())),
            declaration("F", DomainExpr::Function("B".into(), "P".into())),
        ],
        Limits {
            max_elements: 256,
            max_candidates: 100_000,
        },
    )
    .unwrap();
    let nested = registry.get("N").unwrap();

    assert_eq!(
        nested.index(&bottom()),
        nested.index(&app("pair", vec![bottom(), bottom()]))
    );
    assert_eq!(
        registry.get("F").unwrap().index(&bottom()),
        registry
            .get("F")
            .unwrap()
            .index(&app("table", vec![bottom(), bottom(), bottom()]))
    );
    assert_eq!(
        registry.apply_element("F", &bottom(), &bottom()).unwrap(),
        app("pair", vec![bottom(), bottom()])
    );
}

#[test]
fn invalid_declarations_and_exhausted_budgets_are_errors() {
    assert!(Domains::new(&[boolean(), boolean()], Limits::default()).is_err());
    assert!(
        Domains::new(
            &[declaration("", DomainExpr::Flat(vec![]))],
            Limits::default()
        )
        .is_err()
    );
    assert!(
        Domains::new(
            &[declaration("A", DomainExpr::Lifted("A".into()))],
            Limits::default()
        )
        .is_err()
    );
    assert!(
        Domains::new(
            &[declaration("A", DomainExpr::Lifted("missing".into()))],
            Limits::default()
        )
        .is_err()
    );
    for atom in [bottom(), Term::Var("x".into()), Term::Name("x".into())] {
        assert!(
            Domains::new(
                &[declaration("A", DomainExpr::Flat(vec![atom]))],
                Limits::default()
            )
            .is_err()
        );
    }

    assert!(
        Domains::new(
            &[boolean()],
            Limits {
                max_elements: 2,
                max_candidates: 100
            }
        )
        .is_err()
    );
    let declarations = [
        boolean(),
        declaration("F", DomainExpr::Function("B".into(), "B".into())),
    ];

    assert!(
        Domains::new(
            &declarations,
            Limits {
                max_elements: 10,
                max_candidates: 1000
            }
        )
        .is_err()
    );
    assert!(
        Domains::new(
            &declarations,
            Limits {
                max_elements: 20,
                max_candidates: 1
            }
        )
        .is_err()
    );
    assert_eq!(
        Domains::new(
            &declarations,
            Limits {
                max_elements: 11,
                max_candidates: 1000
            }
        )
        .unwrap()
        .get("F")
        .unwrap()
        .elements()
        .len(),
        11
    );
}

