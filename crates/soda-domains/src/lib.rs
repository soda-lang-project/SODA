use soda_ast::{DomainDecl, DomainExpr, Term};
use std::collections::{BTreeMap, BTreeSet};

mod error;
pub use error::{DomainError, DomainErrorKind};
mod build;
mod environment;
mod functions;
pub use environment::Environment;

pub const DEFAULT_MAX_ELEMENTS: usize = 256;
pub const DEFAULT_MAX_CANDIDATES: usize = 65_536;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_elements: usize,
    pub max_candidates: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_elements: DEFAULT_MAX_ELEMENTS,
            max_candidates: DEFAULT_MAX_CANDIDATES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Shape {
    Flat,
    Product(Box<FiniteDomain>, Box<FiniteDomain>),
    Function(Box<FiniteDomain>, Box<FiniteDomain>),
    Lifted(Box<FiniteDomain>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteDomain {
    pub name: String,
    elements: Vec<Term>,
    order: Vec<Vec<bool>>,
    shape: Shape,
}

impl FiniteDomain {
    pub fn elements(&self) -> &[Term] {
        &self.elements
    }

    pub fn bottom(&self) -> &Term {
        &self.elements[0]
    }

    pub fn index(&self, term: &Term) -> Option<usize> {
        if term == &bottom() {
            return Some(0);
        }

        let normalized;
        let term = match (&self.shape, term) {
            (Shape::Product(a, b), Term::App(name, args)) if name == "pair" && args.len() == 2 => {
                return Some(a.index(&args[0])? * b.elements.len() + b.index(&args[1])?);
            }
            (Shape::Function(a, b), Term::App(name, args))
                if name == "table" && args.len() == a.elements.len() =>
            {
                normalized = Term::app(
                    "table",
                    args.iter()
                        .map(|arg| b.index(arg).map(|index| b.elements[index].clone()))
                        .collect::<Option<Vec<_>>>()?,
                );

                &normalized
            }
            (Shape::Lifted(base), Term::App(name, args)) if name == "lift" && args.len() == 1 => {
                return Some(base.index(&args[0])? + 1);
            }
            _ => term,
        };

        self.elements
            .iter()
            .position(|element| element.alpha_equivalent(term))
    }

    pub fn less_equal(&self, left: &Term, right: &Term) -> Result<bool, DomainError> {
        let a = self.require_index(left)?;
        let b = self.require_index(right)?;

        Ok(self.order[a][b])
    }

    fn require_index(&self, term: &Term) -> Result<usize, DomainError> {
        self.index(term).ok_or_else(|| {
            DomainError::new(format!("{term} is not an element of domain {}", self.name))
        })
    }
}

#[derive(Clone, Debug)]
pub struct SemanticFunction {
    pub name: String,
    pub input: String,
    pub output: String,
    table: Vec<usize>,
    input_domain: FiniteDomain,
    output_domain: FiniteDomain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedPoint {
    pub value: Term,
    /// Ascending Kleene chain, starting at bottom and omitting a duplicate end.
    pub chain: Vec<Term>,
}

#[derive(Clone, Debug)]
pub struct Domains {
    domains: BTreeMap<String, FiniteDomain>,
}

impl Domains {
    pub fn new(declarations: &[DomainDecl], limits: Limits) -> Result<Self, DomainError> {
        if limits.max_elements == 0 {
            return Err("domain element budget must be positive".into());
        }

        let mut names = BTreeSet::new();

        for declaration in declarations {
            if declaration.name.trim().is_empty() || !names.insert(&declaration.name) {
                return Err(DomainError::new(format!(
                    "empty or duplicate domain {}",
                    declaration.name
                )));
            }
        }

        let mut result = Self {
            domains: BTreeMap::new(),
        };
        let mut pending = declarations.iter().collect::<Vec<_>>();

        while !pending.is_empty() {
            let before = pending.len();
            let mut next = Vec::new();

            for declaration in pending {
                if let Some(unknown) =
                    dependencies(&declaration.expression)
                        .into_iter()
                        .find(|name| {
                            !declarations
                                .iter()
                                .any(|declaration| declaration.name == *name)
                        })
                {
                    return Err(DomainError::new(format!(
                        "{}: unknown domain {unknown}",
                        declaration.name
                    )));
                }

                if dependencies(&declaration.expression)
                    .iter()
                    .any(|name| !result.domains.contains_key(*name))
                {
                    next.push(declaration);
                    continue;
                }

                let domain = result
                    .build(declaration, limits)
                    .map_err(|error| error.in_declaration(&declaration.name))?;
                result.domains.insert(declaration.name.clone(), domain);
            }

            if next.len() == before {
                return Err(DomainError::new(format!(
                    "{}: cyclic domain dependencies involve {}",
                    next[0].name,
                    next.iter()
                        .map(|declaration| declaration.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }

            pending = next;
        }

        Ok(result)
    }

    pub fn get(&self, name: &str) -> Result<&FiniteDomain, DomainError> {
        self.domains
            .get(name)
            .ok_or_else(|| DomainError::new(format!("unknown domain {name}")))
    }
}

fn bottom() -> Term {
    Term::app("bottom", vec![])
}

fn dependencies(expression: &DomainExpr) -> Vec<&str> {
    match expression {
        DomainExpr::Flat(_) => Vec::new(),
        DomainExpr::Product(a, b) | DomainExpr::Function(a, b) => vec![a, b],
        DomainExpr::Lifted(base) => vec![base],
    }
}

fn require_budget(elements: usize, limits: Limits) -> Result<(), DomainError> {
    if elements > limits.max_elements {
        Err("finite domain exceeds element budget".into())
    } else {
        Ok(())
    }
}

fn monotone(input: &[Vec<bool>], output: &[Vec<bool>], table: &[usize]) -> bool {
    (0..table.len()).all(|a| (0..table.len()).all(|b| !input[a][b] || output[table[a]][table[b]]))
}

fn enumerate_tables(
    input: &FiniteDomain,
    output: &FiniteDomain,
    prefix: &mut Vec<usize>,
    tables: &mut Vec<Vec<usize>>,
    candidates: &mut usize,
    limits: Limits,
) -> Result<(), DomainError> {
    if *candidates >= limits.max_candidates {
        return Err("function space exceeds enumeration budget".into());
    }

    *candidates += 1;
    if prefix.len() == input.elements.len() {
        if monotone(&input.order, &output.order, prefix) {
            require_budget(tables.len() + 1, limits)?;
            tables.push(prefix.clone());
        }

        return Ok(());
    }

    for value in 0..output.elements.len() {
        prefix.push(value);
        // The prefix is monotone; only pairs involving the new entry need checking.

        let last = prefix.len() - 1;
        let valid = (0..prefix.len()).all(|other| {
            (!input.order[other][last] || output.order[prefix[other]][value])
                && (!input.order[last][other] || output.order[value][prefix[other]])
        });

        if valid {
            enumerate_tables(input, output, prefix, tables, candidates, limits)?;
        }

        prefix.pop();
    }

    Ok(())
}
