use super::{Domains, FiniteDomain, Limits, Shape, bottom, enumerate_tables, require_budget};
use crate::DomainError;
use soda_ast::{DomainDecl, DomainExpr, Term};
use std::collections::BTreeSet;

impl Domains {
    pub(super) fn build(
        &self,
        declaration: &DomainDecl,
        limits: Limits,
    ) -> Result<FiniteDomain, DomainError> {
        let (elements, order, shape) = match &declaration.expression {
            DomainExpr::Flat(atoms) => {
                let mut seen = BTreeSet::from([bottom()]);
                let mut elements = vec![bottom()];

                for atom in atoms {
                    if !atom.variables().is_empty()
                        || !atom.free_names().is_empty()
                        || seen.iter().any(|seen| seen.alpha_equivalent(atom))
                    {
                        return Err(
                            "flat domain atoms must be distinct closed terms other than bottom"
                                .into(),
                        );
                    }

                    seen.insert(atom.clone());
                    elements.push(atom.clone());
                }

                require_budget(elements.len(), limits)?;
                let order = (0..elements.len())
                    .map(|a| (0..elements.len()).map(|b| a == 0 || a == b).collect())
                    .collect();

                (elements, order, Shape::Flat)
            }
            DomainExpr::Product(left, right) => {
                let a = self.get(left)?;
                let b = self.get(right)?;
                let size = a
                    .elements
                    .len()
                    .checked_mul(b.elements.len())
                    .ok_or("domain cardinality overflow")?;
                require_budget(size, limits)?;
                let elements = a
                    .elements
                    .iter()
                    .flat_map(|x| {
                        b.elements
                            .iter()
                            .map(move |y| Term::app("pair", vec![x.clone(), y.clone()]))
                    })
                    .collect::<Vec<_>>();
                let order = (0..size)
                    .map(|x| {
                        (0..size)
                            .map(|y| {
                                a.order[x / b.elements.len()][y / b.elements.len()]
                                    && b.order[x % b.elements.len()][y % b.elements.len()]
                            })
                            .collect()
                    })
                    .collect();

                (
                    elements,
                    order,
                    Shape::Product(Box::new(a.clone()), Box::new(b.clone())),
                )
            }
            DomainExpr::Lifted(base) => {
                let base = self.get(base)?;
                require_budget(base.elements.len() + 1, limits)?;
                let elements = std::iter::once(bottom())
                    .chain(
                        base.elements
                            .iter()
                            .map(|value| Term::app("lift", vec![value.clone()])),
                    )
                    .collect::<Vec<_>>();
                let order = (0..elements.len())
                    .map(|a| {
                        (0..elements.len())
                            .map(|b| a == 0 || b != 0 && base.order[a - 1][b - 1])
                            .collect()
                    })
                    .collect();

                (elements, order, Shape::Lifted(Box::new(base.clone())))
            }
            DomainExpr::Function(input, output) => {
                let a = self.get(input)?;
                let b = self.get(output)?;
                let mut tables = Vec::new();
                let mut candidates = 0;
                enumerate_tables(a, b, &mut Vec::new(), &mut tables, &mut candidates, limits)?;
                let elements = tables
                    .iter()
                    .map(|table| {
                        Term::app(
                            "table",
                            table
                                .iter()
                                .map(|&index| b.elements[index].clone())
                                .collect(),
                        )
                    })
                    .collect();
                let order = tables
                    .iter()
                    .map(|x| {
                        tables
                            .iter()
                            .map(|y| x.iter().zip(y).all(|(&u, &v)| b.order[u][v]))
                            .collect()
                    })
                    .collect();

                (
                    elements,
                    order,
                    Shape::Function(Box::new(a.clone()), Box::new(b.clone())),
                )
            }
        };

        Ok(FiniteDomain {
            name: declaration.name.clone(),
            elements,
            order,
            shape,
        })
    }
}
