use crate::DomainError;
use crate::{Domains, FiniteDomain, FixedPoint, SemanticFunction, Shape, monotone};
use soda_ast::{SemanticMap, Term};

impl Domains {
    pub fn compile(&self, map: &SemanticMap) -> Result<SemanticFunction, DomainError> {
        self.compile_inner(map)
            .map_err(|error| error.in_declaration(&map.name))
    }

    fn compile_inner(&self, map: &SemanticMap) -> Result<SemanticFunction, DomainError> {
        if map.name.trim().is_empty() {
            return Err("semantic function name must be nonempty".into());
        }

        let input = self.get(&map.input)?;
        let output = self.get(&map.output)?;
        let mut table = vec![None; input.elements.len()];

        for (left, right) in &map.entries {
            let a = input.require_index(left)?;
            let b = output.require_index(right)?;

            if table[a].replace(b).is_some() {
                return Err(DomainError::new(format!("duplicate mapping for {left}")));
            }
        }

        let table = table
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                DomainError::new("semantic function is not total")
                    .with_kind(crate::DomainErrorKind::NonTotalMap)
            })?;

        if !monotone(&input.order, &output.order, &table) {
            let (a, b) = (0..table.len())
                .flat_map(|a| (0..table.len()).map(move |b| (a, b)))
                .find(|&(a, b)| input.order[a][b] && !output.order[table[a]][table[b]])
                .expect("a nonmonotone table has a witness");

            return Err(DomainError::new(format!(
                "semantic function is not monotone: {} <= {}, but f({}) = {} is not <= f({}) = {}",
                input.elements[a],
                input.elements[b],
                input.elements[a],
                output.elements[table[a]],
                input.elements[b],
                output.elements[table[b]]
            ))
            .with_kind(crate::DomainErrorKind::NonMonotoneMap));
        }

        Ok(SemanticFunction {
            name: map.name.clone(),
            input: map.input.clone(),
            output: map.output.clone(),
            table,
            input_domain: input.clone(),
            output_domain: output.clone(),
        })
    }

    pub fn apply(&self, function: &SemanticFunction, argument: &Term) -> Result<Term, DomainError> {
        let (input, output) = self.function_domains(function)?;
        let index = input.require_index(argument)?;
        let value = *function
            .table
            .get(index)
            .ok_or("function belongs to a different domain registry")?;

        output
            .elements
            .get(value)
            .cloned()
            .ok_or_else(|| "function belongs to a different domain registry".into())
    }

    pub fn least_fixed_point(
        &self,
        function: &SemanticFunction,
    ) -> Result<FixedPoint, DomainError> {
        if function.input != function.output {
            return Err("a fixed point requires an endofunction".into());
        }

        let (domain, _) = self.function_domains(function)?;

        if function.table.len() != domain.elements.len()
            || function
                .table
                .iter()
                .any(|&index| index >= domain.elements.len())
            || !monotone(&domain.order, &domain.order, &function.table)
        {
            return Err("invalid semantic function for this registry".into());
        }

        let mut current = 0;
        let mut chain = vec![domain.elements[0].clone()];

        for _ in 0..domain.elements.len() {
            let next = function.table[current];

            if next == current {
                return Ok(FixedPoint {
                    value: domain.elements[current].clone(),
                    chain,
                });
            }

            if !domain.order[current][next] {
                return Err("fixed-point iteration failed to ascend".into());
            }

            chain.push(domain.elements[next].clone());
            current = next;
        }

        Err("fixed-point iteration exceeded the finite domain height".into())
    }

    fn function_domains(
        &self,
        function: &SemanticFunction,
    ) -> Result<(&FiniteDomain, &FiniteDomain), DomainError> {
        let input = self.get(&function.input)?;
        let output = self.get(&function.output)?;

        if input != &function.input_domain || output != &function.output_domain {
            return Err("function belongs to a different domain registry".into());
        }

        Ok((input, output))
    }

    pub fn apply_element(
        &self,
        domain: &str,
        function: &Term,
        argument: &Term,
    ) -> Result<Term, DomainError> {
        let domain = self.get(domain)?;
        let Shape::Function(input, output) = &domain.shape else {
            return Err(DomainError::new(format!(
                "{} is not a function domain",
                domain.name
            )));
        };

        let function_index = domain.require_index(function)?;
        let argument_index = input.require_index(argument)?;
        let Term::App(_, table) = &domain.elements[function_index] else {
            return Err("invalid internal function representation".into());
        };

        let result = table
            .get(argument_index)
            .ok_or("invalid internal function arity")?;

        Ok(output.elements[output.require_index(result)?].clone())
    }
}
