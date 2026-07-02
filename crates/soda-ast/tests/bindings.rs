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

