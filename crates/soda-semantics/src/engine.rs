use crate::{Config, TraceStep};
use soda_ast::{Rule, Term};
use std::collections::{BTreeMap, BTreeSet};
mod compositional;
mod dispatch;
mod operational;
mod reduction;
mod transitions;
#[derive(Clone, Debug)]
pub struct Engine {
    rules: Vec<Rule>,
    heads: BTreeSet<String>,

    dispatch: BTreeMap<String, BTreeMap<usize, Vec<usize>>>,
}

struct Context<'a> {
    config: &'a Config,
    steps: usize,
    trace: Vec<TraceStep>,
    last_rule: Option<String>,
}

fn within_depth(term: &Term, depth: usize, maximum: usize) -> bool {
    let mut pending = vec![(term, depth)];

    while let Some((term, depth)) = pending.pop() {
        if depth > maximum {
            return false;
        }

        match term {
            Term::App(_, args) => {
                pending.extend(args.iter().map(|arg| (arg, depth.saturating_add(1))))
            }
            Term::Binder { body, .. } => pending.push((body, depth.saturating_add(1))),
            _ => {}
        }
    }

    true
}

fn argument_path(parent: &[usize], index: usize) -> Vec<usize> {
    let mut path = parent.to_vec();
    path.push(index);

    path
}

#[derive(Clone, Debug)]
enum RuleOrigin {
    Primitive(soda_ast::Primitive),
    User(String),
}

impl std::fmt::Display for RuleOrigin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primitive(primitive) => write!(formatter, "builtin:{}", primitive.name()),
            Self::User(name) => formatter.write_str(name),
        }
    }
}

enum Attempt {
    Ready { term: Term, rule: RuleOrigin },
    Idle(Option<String>),
}

enum Search {
    Changed { exceeds_depth: bool },
    Idle(Option<String>),
}

impl Engine {
    fn evaluated_arguments(&self, name: &str, args: &[Term]) -> usize {
        match name {
            "apply"
                if self.heads.contains("apply")
                    && !matches!(args.first(), Some(Term::Binder { .. })) =>
            {
                args.len()
            }
            "if" | "apply" => args.len().min(1),
            "subst" | "alpha_eq" | "fresh" => 0,
            _ => args.len(),
        }
    }
}
