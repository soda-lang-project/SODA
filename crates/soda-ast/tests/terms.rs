use soda_ast::{Substitution, Term, match_term};

#[test]
fn repeated_variables_constrain_matching() {
    let x = Term::Var("x".into());
    let pattern = Term::app("pair", vec![x.clone(), x]);

    assert!(
        match_term(
            &pattern,
            &Term::app("pair", vec![Term::Int(1), Term::Int(2)])
        )
        .is_none()
    );
    assert_eq!(
        match_term(
            &pattern,
            &Term::app("pair", vec![Term::Int(1), Term::Int(1)])
        )
        .unwrap()["x"],
        Term::Int(1)
    );
}

#[test]
fn substitution_is_simultaneous() {
    let bindings = Substitution::from([
        ("x".into(), Term::Var("y".into())),
        ("y".into(), Term::Int(1)),
    ]);

    assert_eq!(
        Term::Var("x".into()).substitute(&bindings),
        Term::Var("y".into())
    );
}

