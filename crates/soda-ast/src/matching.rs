use crate::{Substitution, Term};

/// Matches modulo alpha equivalence. Metavariable solutions must be independent
/// of enclosing bound names.
pub fn match_term(pattern: &Term, term: &Term) -> Option<Substitution> {
    fn visit(
        pattern: &Term,
        term: &Term,
        bindings: &mut Substitution,
        pattern_scope: &mut Vec<String>,
        term_scope: &mut Vec<String>,
    ) -> bool {
        match (pattern, term) {
            (Term::Var(name), _) => {
                if !term_scope.is_empty()
                    && term
                        .free_names()
                        .iter()
                        .any(|name| term_scope.contains(name))
                {
                    return false;
                }

                match bindings.get(name) {
                    Some(value) => value.alpha_equivalent(term),
                    None => {
                        bindings.insert(name.clone(), term.clone());

                        true
                    }
                }
            }
            (Term::Name(a), Term::Name(b)) => match (
                pattern_scope.iter().rposition(|name| name == a),
                term_scope.iter().rposition(|name| name == b),
            ) {
                (Some(a), Some(b)) => a == b,
                (None, None) => a == b,
                _ => false,
            },
            (
                Term::Binder {
                    parameter: a,
                    sort: s,
                    body: x,
                },
                Term::Binder {
                    parameter: b,
                    sort: t,
                    body: y,
                },
            ) if s == t => {
                pattern_scope.push(a.clone());
                term_scope.push(b.clone());
                let matches = visit(x, y, bindings, pattern_scope, term_scope);
                pattern_scope.pop();
                term_scope.pop();

                matches
            }
            (Term::App(a, xs), Term::App(b, ys)) if a == b && xs.len() == ys.len() => xs
                .iter()
                .zip(ys)
                .all(|(x, y)| visit(x, y, bindings, pattern_scope, term_scope)),
            _ => pattern == term,
        }
    }
    let mut bindings = Substitution::new();

    visit(
        pattern,
        term,
        &mut bindings,
        &mut Vec::new(),
        &mut Vec::new(),
    )
    .then_some(bindings)
}
