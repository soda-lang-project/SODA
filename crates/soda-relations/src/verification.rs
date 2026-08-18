use crate::{Condition, Derivation, MAX_DERIVATION_DEPTH, Relations};

impl Relations {
    pub fn verify(&self, derivation: &Derivation) -> Result<(), String> {
        let mut fuel = soda_semantics::DEFAULT_FUEL;
        self.verify_at(derivation, 0, &mut fuel)?;
        let mut pending = vec![derivation];
        let mut judgments = Vec::new();

        while let Some(proof) = pending.pop() {
            judgments.push(proof.conclusion.clone());
            pending.extend(&proof.premises);
        }

        self.schema
            .check_judgments(&judgments)
            .map_err(|error| error.to_string())
    }

    fn verify_at(&self, proof: &Derivation, depth: usize, fuel: &mut usize) -> Result<(), String> {
        if depth > MAX_DERIVATION_DEPTH {
            return Err("derivation exceeds checker depth limit".into());
        }

        let &index = self
            .clause_indices
            .get(&proof.rule)
            .ok_or_else(|| format!("unknown inference rule {}", proof.rule))?;
        let clause = &self.clauses[index];

        if self.variables[index] != proof.substitution.keys().cloned().collect() {
            return Err(format!(
                "{}: substitution must bind exactly the rule variables",
                proof.rule
            ));
        }

        let conditions = self
            .details
            .get(&clause.name)
            .map(|detail| detail.conditions.as_slice())
            .unwrap_or_default();
        let bindings = self
            .details
            .get(&clause.name)
            .map(|detail| detail.bindings.as_slice())
            .unwrap_or_default();
        self.schema
            .check_clause_instantiation_with_bindings(
                clause,
                &proof.substitution,
                conditions,
                bindings,
            )
            .map_err(|error| error.to_string())?;

        if !clause
            .conclusion
            .substitute(&proof.substitution)
            .alpha_equivalent(&proof.conclusion)
            || clause.premises.len() != proof.premises.len()
        {
            return Err(format!("{}: invalid rule instance", proof.rule));
        }

        let mut available = clause.conclusion.variables();
        for position in 0..=clause.premises.len() {
            for binding in bindings
                .iter()
                .filter(|binding| binding.before_premise == position)
            {
                if !binding.value.variables().is_subset(&available) {
                    return Err(format!(
                        "{}: computed binding depends on a later premise or unbound input",
                        proof.rule
                    ));
                }
                let input = binding.value.substitute(&proof.substitution);
                let (value, steps) = self.compute(&input, *fuel);
                *fuel = fuel.saturating_sub(steps);
                if !value.is_some_and(|value| {
                    value.alpha_equivalent(&binding.pattern.substitute(&proof.substitution))
                }) {
                    return Err(format!(
                        "{}: computed binding does not match independently evaluated result",
                        proof.rule
                    ));
                }
                available.extend(binding.pattern.variables());
            }
            let Some(expected) = clause.premises.get(position) else {
                break;
            };
            let child = &proof.premises[position];
            if !expected
                .substitute(&proof.substitution)
                .alpha_equivalent(&child.conclusion)
            {
                return Err(format!(
                    "{}: premise does not match child conclusion",
                    proof.rule
                ));
            }
            self.verify_at(child, depth + 1, fuel)?;
            available.extend(expected.variables());
        }

        if let Some(detail) = self.details.get(&clause.name) {
            for condition in &detail.conditions {
                let condition = condition.substitute(&proof.substitution);

                let (outcome, steps) = self.check_condition(&condition, *fuel);
                *fuel = fuel.saturating_sub(steps);
                if !matches!(outcome, Condition::True) {
                    return Err(format!(
                        "{}: inference side condition is not proved true",
                        proof.rule
                    ));
                }
            }
        }

        Ok(())
    }
}
