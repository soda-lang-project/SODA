use soda_ast::{Substitution, Term, match_term};

fn name(value: &str) -> Term {
    Term::Name(value.into())
}

fn bind(parameter: &str, body: Term) -> Term {
    Term::Binder {
        parameter: parameter.into(),
        sort: "Expr".into(),
        body: Box::new(body),
    }
}

#[test]
fn substitution_renames_binders_and_respects_shadowing() {
    let term = bind("y", Term::app("pair", vec![name("x"), name("y")]));
    let replaced = term.substitute_name("x", &name("y"));

    assert_eq!(replaced.free_names(), ["y".into()].into());
    assert!(replaced.alpha_equivalent(&bind("z", Term::app("pair", vec![name("y"), name("z")]))));
    let shadow = bind("x", bind("x", name("x")));

    assert_eq!(shadow.substitute_name("x", &Term::Int(2)), shadow);
}

#[test]
fn metavariable_insertion_avoids_object_capture() {
    let term = bind("x", Term::Var("body".into()));
    let substituted = term.substitute(&Substitution::from([("body".into(), name("x"))]));

    assert_eq!(substituted.free_names(), ["x".into()].into());
    assert!(substituted.alpha_equivalent(&bind("z", name("x"))));
}

#[test]
fn alpha_equivalence_handles_shadowing_and_free_names() {
    assert!(bind("x", bind("x", name("x"))).alpha_equivalent(&bind("a", bind("b", name("b")))));
    assert!(!bind("x", bind("y", name("x"))).alpha_equivalent(&bind("a", bind("b", name("b")))));
    assert!(!bind("x", name("y")).alpha_equivalent(&bind("y", name("y"))));
}

#[test]
fn matching_and_positions_conservatively_preserve_scopes() {
    let pattern = bind("x", Term::Var("body".into()));

    assert!(match_term(&pattern, &bind("y", name("y"))).is_none());
    assert!(match_term(&bind("x", name("x")), &bind("y", name("y"))).is_some());
    assert!(bind("x", Term::Int(0)).replace(&[0], name("x")).is_none());
    assert_eq!(bind("x", name("x")).at(&[0]), Some(&name("x")));
}

#[test]
fn binder_matching_solves_only_scope_independent_metavariables() {
    let pattern = bind(
        "x",
        Term::app("pair", vec![name("x"), Term::Var("n".into())]),
    );
    let target = bind("y", Term::app("pair", vec![name("y"), Term::Int(3)]));
    let substitution = match_term(&pattern, &target).unwrap();

    assert_eq!(substitution["n"], Term::Int(3));
    assert!(pattern.substitute(&substitution).alpha_equivalent(&target));
    let dependent = bind("y", Term::app("pair", vec![name("y"), name("y")]));

    assert!(match_term(&pattern, &dependent).is_none());
}

#[test]
fn binder_matching_handles_shadowing_free_names_and_repeated_variables() {
    let variable = Term::Var("value".into());
    let pattern = bind(
        "x",
        bind("x", Term::app("pair", vec![name("x"), variable.clone()])),
    );
    let target = bind(
        "a",
        bind("b", Term::app("pair", vec![name("b"), name("x")])),
    );
    let substitution = match_term(&pattern, &target).unwrap();

    assert_eq!(substitution["value"], name("x"));
    assert!(pattern.substitute(&substitution).alpha_equivalent(&target));
    assert!(
        match_term(
            &pattern,
            &bind(
                "a",
                bind("b", Term::app("pair", vec![name("a"), name("x")]))
            )
        )
        .is_none()
    );
    let repeated = Term::app("pair", vec![variable.clone(), bind("x", variable)]);

    assert!(
        match_term(
            &repeated,
            &Term::app("pair", vec![Term::Int(1), bind("y", Term::Int(1))])
        )
        .is_some()
    );
    assert!(
        match_term(
            &repeated,
            &Term::app("pair", vec![Term::Int(1), bind("y", Term::Int(2))])
        )
        .is_none()
    );
}

#[test]
fn successful_binding_matches_reconstruct_the_target_modulo_alpha() {
    let atoms = vec![name("x"), name("y"), Term::Int(1), Term::Var("m".into())];
    let mut terms = atoms.clone();

    for atom in &atoms {
        terms.push(bind("x", atom.clone()));
        terms.push(bind("y", atom.clone()));
        terms.push(bind("x", bind("y", atom.clone())));
        terms.push(bind("y", bind("x", atom.clone())));
    }

    for left in &atoms {
        for right in &atoms {
            terms.push(bind(
                "x",
                Term::app("pair", vec![left.clone(), right.clone()]),
            ));
        }
    }

    for pattern in &terms {
        for target in &terms {
            if let Some(substitution) = match_term(pattern, target) {
                assert!(
                    pattern.substitute(&substitution).alpha_equivalent(target),
                    "{pattern} matched {target} with {substitution:?}"
                );
            }
        }
    }
}

#[test]
fn substitution_preserves_free_names_despite_nested_fresh_name_collisions() {
    let term = bind(
        "y",
        Term::app("pair", vec![name("x"), bind("y_0", name("y"))]),
    );
    let replaced = term.substitute_name("x", &name("y"));

    assert_eq!(replaced.free_names(), ["y".into()].into());
    assert!(replaced.alpha_equivalent(&bind(
        "fresh",
        Term::app("pair", vec![name("y"), bind("other", name("fresh"))])
    )));
    assert_eq!(term.substitute_name("absent", &name("y")), term);
}
