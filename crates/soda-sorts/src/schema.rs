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
