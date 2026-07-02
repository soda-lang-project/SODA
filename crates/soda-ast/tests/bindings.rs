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

