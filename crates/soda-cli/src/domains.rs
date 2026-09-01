use crate::error::CommandError;
use crate::{
    Arguments,
    diagnostics::Context,
    presentation::{Theme, count},
};
use anstyle::AnsiColor;
use soda_ast::{Module, Term};
use soda_domains::{Domains, Limits, SemanticFunction};
use std::collections::BTreeMap;

pub struct CheckedDomains {
    domains: Domains,
    maps: BTreeMap<String, SemanticFunction>,
}

impl CheckedDomains {
    pub fn new(module: &Module) -> Result<Self, CommandError> {
        let domains = Domains::new(&module.domains, Limits::default())?;
        let mut maps = BTreeMap::new();

        for declaration in &module.semantic_maps {
            if declaration.name.trim().is_empty() || maps.contains_key(&declaration.name) {
                return Err(format!("duplicate or empty semantic map {}", declaration.name).into());
            }

            maps.insert(declaration.name.clone(), domains.compile(declaration)?);
        }

        for name in &module.fixed_points {
            let map = maps
                .get(name)
                .ok_or_else(|| format!("unknown fixed-point function {name}"))?;
            domains.least_fixed_point(map)?;
        }

        Ok(Self { domains, maps })
    }

}
