use soda_ast::{Equation, Proof};

use crate::{SortError, TypeChecker, inference::Scope};

/// Sort-checks certificate instances and intermediate expressions in their lexical context.
pub(crate) fn check(
    checker: &TypeChecker,
    equations: &[Equation],
    proof: &Proof,
) -> Result<(), SortError> {
    let mut scope = Scope::new(checker);
    scope.equation(&proof.lhs, &proof.rhs)?;
    let mut current = proof.lhs.clone();

    for step in &proof.steps {
        let axiom = equations
            .iter()
            .find(|equation| equation.name == step.axiom)
            .ok_or_else(|| SortError::new(format!("unknown axiom `{}`", step.axiom)))?;
        let mut template = Scope::new(checker);
        template.equation(&axiom.lhs, &axiom.rhs)?;
        let expected = template.variable_sorts()?;

        if expected.len() != step.substitution.len() || expected.keys().ne(step.substitution.keys())
        {
            return Err(SortError::new(
                "axiom substitution must bind exactly its metavariables".into(),
            ));
        }

        scope.enter_position(&current, &step.path)?;
        for (name, sort) in expected {
            scope.expected(&step.substitution[&name], &sort)?;
        }

        scope.clear_names();
        let target = if step.reverse { &axiom.lhs } else { &axiom.rhs };

        current = current
            .replace(&step.path, target.substitute(&step.substitution))
            .ok_or_else(|| SortError::new("invalid proof position".into()))?;
        scope.equation(&proof.lhs, &current)?;
    }

    scope.finish()
}
