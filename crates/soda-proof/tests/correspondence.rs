use soda_ast::{DomainDecl, DomainExpr, Rule, SemanticMap, Term};
use soda_domains::{Domains, Limits};
use soda_proof::{Correspondence, check_correspondence};
use soda_semantics::{Config, Engine, Limit, Status};

fn atom(name: &str) -> Term {
    Term::app(name, vec![])
}

fn registry(values: Vec<Term>) -> Domains {
    Domains::new(
        &[DomainDecl {
            name: "B".into(),
            expression: DomainExpr::Flat(values),
        }],
        Limits::default(),
    )
    .unwrap()
}

fn booleans() -> Domains {
    registry(vec![Term::Bool(false), Term::Bool(true)])
}

fn identity(domains: &Domains) -> soda_domains::SemanticFunction {
    domains
        .compile(&SemanticMap {
            name: "identity".into(),
            input: "B".into(),
            output: "B".into(),
            entries: domains
                .get("B")
                .unwrap()
                .elements()
                .iter()
                .map(|term| (term.clone(), term.clone()))
                .collect(),
        })
        .unwrap()
}

fn engine(rhs: Term) -> Engine {
    Engine::new(vec![Rule {
        name: "op".into(),
        lhs: Term::app("run", vec![Term::Var("x".into())]),
        rhs,
        condition: None,
    }])
    .unwrap()
}

#[test]
fn checks_every_finite_value_including_encoded_bottom() {
    let domains = booleans();
    let result = check_correspondence(
        &engine(Term::Var("x".into())),
        &domains,
        &identity(&domains),
        "run",
        &Config::default(),
    )
    .unwrap();

    assert_eq!(result, Correspondence::Established { cases: 3 });
}

#[test]
fn wrong_table_produces_concrete_witness() {
    let domains = booleans();
    let constant = domains
        .compile(&SemanticMap {
            name: "constant".into(),
            input: "B".into(),
            output: "B".into(),
            entries: domains
                .get("B")
                .unwrap()
                .elements()
                .iter()
                .map(|term| (term.clone(), Term::Bool(true)))
                .collect(),
        })
        .unwrap();
    let result = check_correspondence(
        &engine(Term::Var("x".into())),
        &domains,
        &constant,
        "run",
        &Config::default(),
    )
    .unwrap();

    assert_eq!(
        result,
        Correspondence::Counterexample {
            input: atom("bottom"),
            expected: Term::Bool(true),
            actual: atom("bottom"),
            status: Status::NormalForm
        }
    );
}

#[test]
fn fuel_exhaustion_is_unknown() {
    let domains = booleans();
    let result = check_correspondence(
        &engine(Term::app("run", vec![Term::Var("x".into())])),
        &domains,
        &identity(&domains),
        "run",
        &Config {
            fuel: 2,
            ..Config::default()
        },
    )
    .unwrap();

    assert_eq!(
        result,
        Correspondence::Unknown {
            input: atom("bottom"),
            status: Status::Limit(Limit::Fuel)
        }
    );
}

#[test]
fn out_of_domain_and_stuck_results_violate_normal_form_contract() {
    let domains = booleans();
    let map = identity(&domains);
    let result = check_correspondence(
        &engine(Term::Int(0)),
        &domains,
        &map,
        "run",
        &Config::default(),
    )
    .unwrap();

    assert!(matches!(
        result,
        Correspondence::Counterexample {
            actual: Term::Int(0),
            ..
        }
    ));
    let stuck = Engine::new(vec![Rule {
        name: "only_true".into(),
        lhs: Term::app("run", vec![Term::Bool(true)]),
        rhs: Term::Bool(true),
        condition: None,
    }])
    .unwrap();

    assert!(matches!(
        check_correspondence(&stuck, &domains, &map, "run", &Config::default()).unwrap(),
        Correspondence::Counterexample {
            status: Status::Stuck(_),
            ..
        }
    ));
}

#[test]
fn functions_compiled_for_another_registry_are_errors() {
    let domains = booleans();
    let other = registry(vec![Term::Int(0), Term::Int(1)]);

    assert!(
        check_correspondence(
            &engine(Term::Var("x".into())),
            &other,
            &identity(&domains),
            "run",
            &Config::default()
        )
        .unwrap_err()
        .to_string()
        .contains("different domain registry")
    );
}

#[test]
fn compares_canonical_domain_values_instead_of_raw_encodings() {
    let domains = Domains::new(
        &[
            DomainDecl {
                name: "B".into(),
                expression: DomainExpr::Flat(vec![Term::Bool(false), Term::Bool(true)]),
            },
            DomainDecl {
                name: "P".into(),
                expression: DomainExpr::Product("B".into(), "B".into()),
            },
        ],
        Limits::default(),
    )
    .unwrap();
    let map = domains
        .compile(&SemanticMap {
            name: "bottom_map".into(),
            input: "B".into(),
            output: "P".into(),
            entries: domains
                .get("B")
                .unwrap()
                .elements()
                .iter()
                .map(|term| (term.clone(), atom("bottom")))
                .collect(),
        })
        .unwrap();

    assert_eq!(
        check_correspondence(
            &engine(atom("bottom")),
            &domains,
            &map,
            "run",
            &Config::default()
        )
        .unwrap(),
        Correspondence::Established { cases: 3 }
    );
}

#[test]
fn arithmetic_errors_are_witnesses_and_depth_limits_are_unknown() {
    let domains = booleans();
    let map = identity(&domains);
    let error = engine(Term::app("div", vec![Term::Int(1), Term::Int(0)]));

    assert!(matches!(
        check_correspondence(&error, &domains, &map, "run", &Config::default()).unwrap(),
        Correspondence::Counterexample {
            status: Status::Error(_),
            ..
        }
    ));
    assert!(matches!(
        check_correspondence(
            &engine(Term::Var("x".into())),
            &domains,
            &map,
            "run",
            &Config {
                max_depth: 0,
                ..Config::default()
            }
        )
        .unwrap(),
        Correspondence::Unknown {
            status: Status::Limit(Limit::Depth),
            ..
        }
    ));
}
