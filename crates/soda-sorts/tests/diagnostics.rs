use soda_ast::{Module, Signature, SignatureKind, Term};
use soda_sorts::{TypeChecker, validate_module_all};

#[test]
fn argument_errors_preserve_actual_expected_and_the_smallest_failing_term() {
    let checker = TypeChecker::new(&Module::default()).unwrap();
    let term = Term::app("add", vec![Term::Int(1), Term::Bool(false)]);
    let error = checker.check_term(&term).unwrap_err();

    assert_eq!(error.expected.as_deref(), Some("Int"));
    assert_eq!(error.found.as_deref(), Some("Bool"));
    assert_eq!(error.term.as_deref(), Some(&Term::Bool(false)));
    let binder = Term::Binder {
        parameter: "x".into(),
        sort: "Int".into(),
        body: Box::new(Term::Name("x".into())),
    };
    let error = checker
        .check_term(&Term::app("apply", vec![binder, Term::Bool(true)]))
        .unwrap_err();

    assert_eq!(error.expected.as_deref(), Some("Int"));
    assert_eq!(error.found.as_deref(), Some("Bool"));
}

#[test]
fn independent_errors_accumulate_but_invalid_schemas_do_not_cascade() {
    let mut module = Module {
        signatures: vec![Signature {
            name: "f".into(),
            arguments: vec!["Int".into()],
            result: "Int".into(),
            kind: SignatureKind::Function,
        }],
        evaluations: vec![
            Term::app("f", vec![Term::Bool(true)]),
            Term::app("add", vec![Term::Bool(false), Term::Int(3)]),
        ],
        ..Module::default()
    };
    let errors = validate_module_all(&module).unwrap_err();

    assert_eq!(errors.len(), 2);
    module.signatures[0].result = "Missing".into();
    let errors = validate_module_all(&module).unwrap_err();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].declaration.as_deref(), Some("f"));
    assert_eq!(errors[0].symbol.as_deref(), Some("Missing"));
}

#[test]
fn schema_failures_identify_the_constructor_or_notation_declaration() {
    let module = Module {
        signatures: vec![Signature {
            name: "bad_constructor".into(),
            arguments: vec![],
            result: "Int".into(),
            kind: SignatureKind::Constructor,
        }],
        ..Module::default()
    };
    let error = TypeChecker::new(&module).unwrap_err();

    assert_eq!(error.declaration.as_deref(), Some("bad_constructor"));
    let module = Module {
        notations: vec![soda_ast::NotationDecl {
            judgment: "missing".into(),
            unicode: "$0 ⇓ $1".into(),
            ascii: "$0 ==> $1".into(),
        }],
        ..Module::default()
    };
    let error = TypeChecker::new(&module).unwrap_err();

    assert_eq!(error.declaration.as_deref(), Some("missing"));
    assert_eq!(error.symbol.as_deref(), Some("missing"));
}

#[test]
fn ambiguous_variables_and_non_application_goals_retain_source_terms() {
    let checker = TypeChecker::new(&Module::default()).unwrap();
    let variable = Term::Var("unconstrained".into());
    let error = checker.check_equation(&variable, &variable).unwrap_err();

    assert_eq!(error.term.as_deref(), Some(&variable));
    let error = checker.check_judgment(&Term::Int(1)).unwrap_err();

    assert_eq!(error.term.as_deref(), Some(&Term::Int(1)));
}
