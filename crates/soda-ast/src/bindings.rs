use std::collections::BTreeSet;

use crate::Term;

impl Term {
    pub fn free_names(&self) -> BTreeSet<String> {
        match self {
            Self::Name(name) => BTreeSet::from([name.clone()]),
            Self::Binder {
                parameter, body, ..
            } => {
                let mut names = body.free_names();
                names.remove(parameter);

                names
            }
            Self::App(_, args) => args.iter().flat_map(Self::free_names).collect(),
            _ => BTreeSet::new(),
        }
    }

    fn all_names(&self) -> BTreeSet<String> {
        let mut names = self.free_names();

        match self {
            Self::Binder {
                parameter, body, ..
            } => {
                names.insert(parameter.clone());
                names.extend(body.all_names());
            }
            Self::App(_, args) => names.extend(args.iter().flat_map(Self::all_names)),
            _ => {}
        }

        names
    }

    pub(crate) fn freshen(
        parameter: &str,
        body: &Self,
        avoid: &BTreeSet<String>,
    ) -> (String, Self) {
        if !avoid.contains(parameter) {
            return (parameter.to_owned(), body.clone());
        }

        let mut unavailable = body.all_names();
        unavailable.extend(avoid.iter().cloned());
        let mut index = 0;
        let fresh = loop {
            let candidate = format!("{parameter}_{index}");

            if !unavailable.contains(&candidate) {
                break candidate;
            }

            index += 1;
        };

        let renamed = body.substitute_name(parameter, &Self::Name(fresh.clone()));

        (fresh, renamed)
    }

    pub fn substitute_name(&self, name: &str, replacement: &Self) -> Self {
        match self {
            Self::Name(current) if current == name => replacement.clone(),
            Self::Binder { parameter, .. } if parameter == name => self.clone(),
            Self::Binder {
                parameter,
                sort,
                body,
            } => {
                if !body.free_names().contains(name) {
                    return self.clone();
                }

                let (parameter, body) = Self::freshen(parameter, body, &replacement.free_names());

                Self::Binder {
                    parameter,
                    sort: sort.clone(),
                    body: Box::new(body.substitute_name(name, replacement)),
                }
            }
            Self::App(head, args) => Self::app(
                head,
                args.iter()
                    .map(|arg| arg.substitute_name(name, replacement))
                    .collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn alpha_equivalent(&self, other: &Self) -> bool {
        fn visit(
            left: &Term,
            right: &Term,
            left_scope: &mut Vec<String>,
            right_scope: &mut Vec<String>,
        ) -> bool {
            match (left, right) {
                (Term::Name(a), Term::Name(b)) => {
                    match (
                        left_scope.iter().rposition(|n| n == a),
                        right_scope.iter().rposition(|n| n == b),
                    ) {
                        (Some(x), Some(y)) => x == y,
                        (None, None) => a == b,
                        _ => false,
                    }
                }
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
                    left_scope.push(a.clone());
                    right_scope.push(b.clone());
                    let same = visit(x, y, left_scope, right_scope);
                    left_scope.pop();
                    right_scope.pop();

                    same
                }
                (Term::App(a, xs), Term::App(b, ys)) if a == b && xs.len() == ys.len() => xs
                    .iter()
                    .zip(ys)
                    .all(|(x, y)| visit(x, y, left_scope, right_scope)),
                _ => left == right,
            }
        }
        visit(self, other, &mut Vec::new(), &mut Vec::new())
    }
}
