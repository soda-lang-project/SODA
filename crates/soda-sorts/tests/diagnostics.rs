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

