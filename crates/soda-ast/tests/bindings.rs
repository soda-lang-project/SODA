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

