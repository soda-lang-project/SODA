mod sorts;
use std::collections::{BTreeMap, BTreeSet};

use soda_ast::{Clause, Module, SignatureKind, Substitution, Term};

use crate::{Sort, SortError, inference::Scope};

#[derive(Clone, Debug)]
pub struct TypeChecker {
    typed: bool,
    sorts: BTreeSet<String>,
    pub(crate) signatures: BTreeMap<String, (Vec<Sort>, Sort)>,
    pub(crate) judgments: BTreeMap<String, Vec<Sort>>,
    pub(crate) inductive_constructors: BTreeSet<String>,
}

impl TypeChecker {
    pub fn new(module: &Module) -> Result<Self, SortError> {
        if !module.imports.is_empty() {
            return Err(SortError::new(format!(
                "unresolved imports: {}; load the source with the module loader before checking it",
                module.imports.join(", ")
            )));
        }
        let mut checker = Self {
            typed: !module.sorts.is_empty()
                || !module.signatures.is_empty()
                || !module.judgments.is_empty()
                || crate::bindings_require_sort_check(module),
            sorts: ["Int", "Bool", "Name"]
                .into_iter()
                .map(String::from)
                .collect(),
            signatures: BTreeMap::new(),
            judgments: BTreeMap::new(),
            inductive_constructors: BTreeSet::new(),
        };

        for sort in &module.sorts {
            if sort.name.trim().is_empty()
                || sort.name.starts_with("Fn(")
                || !checker.sorts.insert(sort.name.clone())
            {
                return Err(
                    SortError::new(format!("invalid or duplicate sort `{}`", sort.name))
                        .with_kind(crate::SortErrorKind::DuplicateDeclaration)
                        .in_declaration(&sort.name)
                        .symbol(&sort.name),
                );
            }
        }

        let mut symbols = BTreeSet::new();

        for signature in &module.signatures {
            register(&mut symbols, &signature.name)
                .map_err(|error| error.in_declaration(&signature.name))?;
            let arguments: Vec<Sort> = signature
                .arguments
                .iter()
                .map(|s| checker.parse_sort(s))
                .collect::<Result<_, _>>()
                .map_err(|error: SortError| error.in_declaration(&signature.name))?;
            let result = checker
                .parse_sort(&signature.result)
                .map_err(|error| error.in_declaration(&signature.name))?;

            if signature.kind == SignatureKind::Constructor {
                let Sort::Named(result_name) = &result else {
                    return Err(SortError::new(format!(
                        "constructor `{}` must return a declared data sort",
                        signature.name
                    ))
                    .in_declaration(&signature.name));
                };

                if ["Int", "Bool", "Name"].contains(&result_name.as_str()) {
                    return Err(SortError::new(format!(
                        "constructor `{}` cannot extend builtin sort `{result_name}`",
                        signature.name
                    ))
                    .in_declaration(&signature.name));
                }

                if module
                    .sorts
                    .iter()
                    .any(|sort| sort.name == *result_name && sort.inductive)
                {
                    if arguments
                        .iter()
                        .any(|sort| matches!(sort, Sort::Function(_, _)))
                    {
                        return Err(SortError::new(format!(
                            "inductive constructor `{}` requires named argument sorts",
                            signature.name
                        ))
                        .in_declaration(&signature.name));
                    }

                    checker
                        .inductive_constructors
                        .insert(signature.name.clone());
                }
            }

            checker
                .signatures
                .insert(signature.name.clone(), (arguments, result));
        }

        for judgment in &module.judgments {
            register(&mut symbols, &judgment.name)
                .map_err(|error| error.in_declaration(&judgment.name))?;
            let arguments = judgment
                .arguments
                .iter()
                .map(|s| checker.parse_sort(s))
                .collect::<Result<_, _>>()
                .map_err(|error: SortError| error.in_declaration(&judgment.name))?;
            checker.judgments.insert(judgment.name.clone(), arguments);
        }

        for notation in &module.notations {
            if !checker.judgments.contains_key(&notation.judgment) {
                return Err(SortError::new(format!(
                    "notation references undeclared judgment `{}`",
                    notation.judgment
                ))
                .in_declaration(&notation.judgment)
                .symbol(&notation.judgment));
            }
        }

        Ok(checker)
    }

    pub fn is_typed(&self) -> bool {
        self.typed
    }

    pub fn check_term(&self, term: &Term) -> Result<Sort, SortError> {
        let mut scope = Scope::new(self);
        let ty = scope.term(term)?;
        scope.finish()?;

        scope.materialize(&ty)
    }

    pub fn check_judgment(&self, term: &Term) -> Result<(), SortError> {
        let mut scope = Scope::new(self);
        scope.judgment(term)?;

        scope.finish()
    }

    /// Check related goals in a shared runtime metavariable context.
    pub fn check_judgments(&self, terms: &[Term]) -> Result<(), SortError> {
        if !self.typed && !terms.iter().any(crate::requires_sort_check) {
            return Ok(());
        }

        let mut scope = Scope::new(self);

        for term in terms {
            scope.judgment(term)?;
        }

        scope.finish()
    }

    /// Checks a clause instance against its universally quantified variable sorts.
    pub fn check_clause_instantiation(
        &self,
        clause: &Clause,
        substitution: &Substitution,
        conditions: &[Term],
    ) -> Result<(), SortError> {
        self.check_clause_instantiation_with_bindings(clause, substitution, conditions, &[])
    }

    pub fn check_clause_instantiation_with_bindings(
        &self,
        clause: &Clause,
        substitution: &Substitution,
        conditions: &[Term],
        bindings: &[soda_ast::ComputedBinding],
    ) -> Result<(), SortError> {
        if !self.typed
            && !bindings.iter().any(|binding| {
                crate::requires_sort_check(&binding.pattern)
                    || crate::requires_sort_check(&binding.value)
            })
            && !substitution.values().any(crate::requires_sort_check)
            && !crate::requires_sort_check(&clause.conclusion)
            && !clause
                .premises
                .iter()
                .chain(conditions)
                .any(crate::requires_sort_check)
        {
            return Ok(());
        }

        let mut template = Scope::new(self);
        template.judgment(&clause.conclusion)?;
        for premise in &clause.premises {
            template.judgment(premise)?;
        }

        for condition in conditions {
            template.boolean(condition)?;
        }

        for binding in bindings {
            template.equation(&binding.pattern, &binding.value)?;
        }
        let expected = template.variable_sorts()?;

        if expected.len() != substitution.len() || expected.keys().ne(substitution.keys()) {
            return Err(SortError::new(
                "clause substitution must bind exactly its metavariables".into(),
            ));
        }

        let mut instance = Scope::new(self);

        for (name, sort) in expected {
            instance.expected(&substitution[&name], &sort)?;
        }

        instance.judgment(&clause.conclusion.substitute(substitution))?;
        for premise in &clause.premises {
            instance.judgment(&premise.substitute(substitution))?;
        }

        for condition in conditions {
            instance.boolean(&condition.substitute(substitution))?;
        }

        for binding in bindings {
            instance.equation(
                &binding.pattern.substitute(substitution),
                &binding.value.substitute(substitution),
            )?;
        }
        instance.finish()
    }

    pub fn check_equation(&self, lhs: &Term, rhs: &Term) -> Result<(), SortError> {
        let mut scope = Scope::new(self);
        scope.equation(lhs, rhs)?;

        scope.finish()
    }

    pub fn check_equation_with_variable_sort(
        &self,
        lhs: &Term,
        rhs: &Term,
        variable: &str,
        sort: &str,
    ) -> Result<(), SortError> {
        let mut scope = Scope::new(self);
        scope.expected(&Term::Var(variable.into()), &self.parse_sort(sort)?)?;
        scope.equation(lhs, rhs)?;

        scope.finish()
    }

    /// Keeps axiom variables separate from metavariables in replacement terms.
    pub fn check_instantiation(
        &self,
        lhs: &Term,
        rhs: &Term,
        substitution: &Substitution,
    ) -> Result<(), SortError> {
        let mut template = Scope::new(self);
        template.equation(lhs, rhs)?;
        let expected = template.variable_sorts()?;

        if expected.len() != substitution.len() || expected.keys().ne(substitution.keys()) {
            return Err(SortError::new(
                "axiom substitution must bind exactly its metavariables".into(),
            ));
        }

        let mut replacements = Scope::new(self);

        for (name, sort) in expected {
            replacements.expected(&substitution[&name], &sort)?;
        }

        replacements.finish()
    }
}

fn register(symbols: &mut BTreeSet<String>, name: &str) -> Result<(), SortError> {
    if name.trim().is_empty() {
        return Err(SortError::new("symbol names cannot be empty".into()));
    }

    if soda_ast::Primitive::from_name(name).is_some() {
        return Err(SortError::new(format!(
            "symbol `{name}` is reserved for a builtin"
        )));
    }

    if !symbols.insert(name.into()) {
        return Err(
            SortError::new(format!("duplicate declaration of symbol `{name}`"))
                .with_kind(crate::SortErrorKind::DuplicateDeclaration),
        );
    }

    Ok(())
}
