use soda_ast::{Module, Rule, Substitution, Term};
use std::collections::{BTreeMap, BTreeSet};

use crate::validation::validate_term;
use crate::{VerificationError, unify, validate_module};

/// A syntactic peak with variables freshened independently of other pairs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriticalPair {
    pub outer_rule: String,
    pub inner_rule: String,
    /// The inner contraction position; the outer contraction is at the root.
    pub position: Vec<usize>,
    pub source: Term,
    pub outer_reduct: Term,
    pub inner_reduct: Term,
}

/// Syntactic overlap information, not a confluence or joinability verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriticalPairAnalysis {
    pub pairs: Vec<CriticalPair>,
    pub excluded_conditional_rules: Vec<String>,
    pub limitations: Vec<String>,
}

/// Enumerates nonvariable overlaps of unconditional rules, including self-overlaps.
/// Omits equal reducts and treats symbols as uninterpreted. Does not check joinability.
pub fn critical_pairs(rules: &[Rule]) -> Result<CriticalPairAnalysis, VerificationError> {
    for rule in rules {
        validate_term(&rule.lhs, &rule.name)?;
        validate_term(&rule.rhs, &rule.name)?;

        if let Some(condition) = &rule.condition {
            validate_term(condition, &rule.name)?;
        }
    }

    validate_module(&Module {
        rules: rules.to_vec(),
        ..Module::default()
    })?;
    let mut report = CriticalPairAnalysis {
        pairs: Vec::new(),
        excluded_conditional_rules: rules.iter().filter(|rule| rule.condition.is_some()).map(|rule| rule.name.clone()).collect(),
        limitations: vec![
            "Only unconditional syntactic rewriting is analysed; guarded rules are excluded.".into(),
            "Symbols are uninterpreted; builtin evaluation and evaluation strategies are not modelled.".into(),
            "Variable-position overlaps are omitted, including those relevant to non-left-linear rules.".into(),
            "Binder bodies and scope-dependent binder unification are conservatively omitted.".into(),
            "Reducts are not checked for joinability; no confluence conclusion follows from this report.".into(),
        ],
    };

    let inner_rules: Vec<_> = rules
        .iter()
        .filter(|rule| rule.condition.is_none())
        .map(|rule| (rule, freshen(rule, "inner")))
        .collect();
    let mut dispatch: BTreeMap<(&str, usize), Vec<usize>> = BTreeMap::new();

    for (index, (_, (lhs, _))) in inner_rules.iter().enumerate() {
        if let Term::App(head, args) = lhs {
            dispatch.entry((head, args.len())).or_default().push(index);
        }
    }

    for outer in rules.iter().filter(|rule| rule.condition.is_none()) {
        let (outer_lhs, outer_rhs) = freshen(outer, "outer");
        let mut positions = Vec::new();
        nonvariable_positions(&outer_lhs, &mut Vec::new(), &mut positions);

        let mut candidates = BTreeSet::new();

        for position in &positions {
            if let Some(Term::App(head, args)) = outer_lhs.at(position)
                && let Some(indices) = dispatch.get(&(head.as_str(), args.len()))
            {
                candidates.extend(indices.iter().copied());
            }
        }

        for index in candidates {
            let (inner, (inner_lhs, inner_rhs)) = &inner_rules[index];

            for position in &positions {
                let Some(selected) = outer_lhs.at(position) else {
                    continue;
                };

                let Some(substitution) = unify(selected, inner_lhs) else {
                    continue;
                };

                let outer_reduct = outer_rhs.substitute(&substitution);
                let Some(replaced) = outer_lhs.replace(position, inner_rhs.clone()) else {
                    continue;
                };

                let inner_reduct = replaced.substitute(&substitution);

                if outer_reduct != inner_reduct {
                    report.pairs.push(CriticalPair {
                        outer_rule: outer.name.clone(),
                        inner_rule: inner.name.clone(),
                        position: position.clone(),
                        source: outer_lhs.substitute(&substitution),
                        outer_reduct,
                        inner_reduct,
                    });
                }
            }
        }
    }

    Ok(report)
}

fn freshen(rule: &Rule, namespace: &str) -> (Term, Term) {
    // Every source variable is renamed into disjoint inner and outer namespaces.

    let substitution = rule
        .lhs
        .variables()
        .into_iter()
        .enumerate()
        .map(|(index, name)| (name, Term::Var(format!("__soda_{namespace}_{index}"))))
        .collect::<Substitution>();

    (
        rule.lhs.substitute(&substitution),
        rule.rhs.substitute(&substitution),
    )
}

fn nonvariable_positions(term: &Term, path: &mut Vec<usize>, positions: &mut Vec<Vec<usize>>) {
    if matches!(term, Term::Var(_)) {
        return;
    }

    positions.push(path.clone());

    if let Term::App(_, args) = term {
        for (index, argument) in args.iter().enumerate() {
            path.push(index);
            nonvariable_positions(argument, path, positions);
            path.pop();
        }
    }
}
