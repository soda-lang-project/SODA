use super::{Engine, within_depth};
use crate::{MAX_DEPTH, ValidationError};
use soda_ast::{Rule, Term};
use std::collections::{BTreeMap, BTreeSet};

impl Engine {
    pub fn new(rules: Vec<Rule>) -> Result<Self, ValidationError> {
        let mut names = BTreeSet::new();
        let mut heads = BTreeSet::new();

        for rule in &rules {
            if !within_depth(&rule.lhs, 0, MAX_DEPTH)
                || !within_depth(&rule.rhs, 0, MAX_DEPTH)
                || rule
                    .condition
                    .as_ref()
                    .is_some_and(|term| !within_depth(term, 0, MAX_DEPTH))
            {
                return Err(ValidationError::new(
                    &rule.name,
                    format!("exceeds maximum depth {MAX_DEPTH}"),
                ));
            }

            if rule.name.is_empty() || !names.insert(rule.name.clone()) {
                return Err(ValidationError::new(
                    &rule.name,
                    "has an empty or duplicate name",
                ));
            }

            if !matches!(rule.lhs, Term::App(_, _)) {
                return Err(ValidationError::new(
                    &rule.name,
                    "requires a constructor-rooted left-hand side",
                ));
            }

            if let Term::App(head, _) = &rule.lhs {
                if soda_ast::Primitive::from_name(head).is_some() && head != "apply" {
                    return Err(ValidationError::new(
                        &rule.name,
                        format!("redefines reserved builtin {head}"),
                    ));
                }

                heads.insert(head.clone());
            }

            let bound = rule.lhs.variables();
            let mut used = rule.rhs.variables();

            if let Some(condition) = &rule.condition {
                used.extend(condition.variables());
            }

            if let Some(variable) = used.difference(&bound).next() {
                return Err(ValidationError::new(
                    &rule.name,
                    format!("uses unbound variable ?{variable}"),
                ));
            }
        }

        let mut dispatch: BTreeMap<String, BTreeMap<usize, Vec<usize>>> = BTreeMap::new();

        for (index, rule) in rules.iter().enumerate() {
            if let Term::App(head, args) = &rule.lhs {
                dispatch
                    .entry(head.clone())
                    .or_default()
                    .entry(args.len())
                    .or_default()
                    .push(index);
            }
        }

        Ok(Self {
            rules,
            heads,
            dispatch,
        })
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}
