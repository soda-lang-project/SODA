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

#[test]
fn positions_are_checked_and_replacement_preserves_context() {
    let term = Term::app(
        "pair",
        vec![Term::Int(1), Term::app("some", vec![Term::Int(2)])],
    );

    assert_eq!(term.at(&[1, 0]), Some(&Term::Int(2)));
    assert!(term.replace(&[99], Term::Int(0)).is_none());
    assert!(term.replace(&[0, 0], Term::Int(0)).is_none());
    let updated = term.replace(&[1, 0], Term::Int(3)).unwrap();

    assert_eq!(updated.to_string(), "pair(1, some(3))");
    assert_eq!(term.to_string(), "pair(1, some(2))");
}

#[test]
fn every_invalid_position_returns_none_including_extreme_indices() {
    let term = Term::app("tree", vec![Term::app("leaf", vec![]), Term::Bool(true)]);

    for path in [
        vec![usize::MAX],
        vec![0, 0],
        vec![1, 0],
        vec![2],
        vec![0, usize::MAX],
    ] {
        assert_eq!(term.at(&path), None);
        assert_eq!(term.replace(&path, Term::Int(7)), None);
    }

    assert_eq!(term.at(&[]), Some(&term));
    assert_eq!(term.replace(&[], Term::Int(7)), Some(Term::Int(7)));
    assert_eq!(Term::Int(1).replace(&[0], Term::Int(7)), None);
}

#[test]
fn matching_distinguishes_constructors_arities_and_literal_types() {
    let cases = [
        (Term::app("a", vec![]), Term::app("b", vec![])),
        (Term::app("a", vec![]), Term::app("a", vec![Term::Int(1)])),
        (Term::Int(1), Term::Bool(true)),
        (Term::Bool(false), Term::Bool(true)),
        (Term::Int(1), Term::Var("x".into())),
    ];

    for (pattern, target) in cases {
        assert!(match_term(&pattern, &target).is_none());
    }

    let target = Term::app("a", vec![Term::Int(1)]);

    assert_eq!(match_term(&target, &target), Some(Substitution::new()));
    assert_eq!(
        match_term(&Term::Var("x".into()), &target).unwrap()["x"],
        target
    );
}

#[test]
fn variable_collection_and_substitution_recurse_through_constructor_arguments() {
    let term = Term::app(
        "pair",
        vec![
            Term::Var("x".into()),
            Term::app("nested", vec![Term::Var("y".into()), Term::Var("x".into())]),
        ],
    );

    assert_eq!(term.variables(), ["x".into(), "y".into()].into());
    let substitution = [
        ("x".into(), Term::Var("y".into())),
        ("y".into(), Term::Int(2)),
    ]
    .into();

    assert_eq!(
        term.substitute(&substitution).to_string(),
        "pair(?y, nested(2, ?y))"
    );
    assert_eq!(term.substitute(&Substitution::new()), term);
}
