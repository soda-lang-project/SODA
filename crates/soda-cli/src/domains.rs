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

    pub fn show(&self, args: &Arguments, context: &mut Context) -> Result<bool, CommandError> {
        let theme = Theme::new(args);
        theme.heading("finite denotational domains", &context.module);
        if let Some(input) = args.term.as_deref() {
            let Term::App(name, values) = context.parse_input(input)? else {
                return Err("domain application must be map_name(value)".into());
            };

            if values.len() != 1 {
                return Err("domain application needs exactly one value".into());
            }

            let function = self
                .maps
                .get(&name)
                .ok_or_else(|| format!("unknown semantic map {name}"))?;
            let result = self.domains.apply(function, &values[0])?;
            theme.goal("semantic application", input);
            theme.message("semantic value established", AnsiColor::BrightGreen);
            theme.field(
                "result",
                &format!("{input} = {}", theme.term(&context.module, &result)),
            );

            theme.end();

            return Ok(true);
        }

        let module = &context.module;

        for declaration in &module.domains {
            let domain = self.domains.get(&declaration.name)?;
            theme.goal(
                &format!(
                    "domain {}: {} elements",
                    domain.name,
                    domain.elements().len()
                ),
                "finite pointed domain",
            );
            theme.message("domain checked", AnsiColor::BrightGreen);
            theme.field("bottom", &theme.term(module, domain.bottom()));
            if args.config.trace {
                theme.field(
                    "elements",
                    &domain
                        .elements()
                        .iter()
                        .map(|term| theme.term(module, term))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }

            theme.end();
        }

        for map in self.maps.values() {
            theme.goal(
                &format!("map {}", map.name),
                &format!("{} {} {}", map.input, theme.arrow(), map.output),
            );
            theme.message("totality and monotonicity checked", AnsiColor::BrightGreen);
            theme.end();
        }

        for name in &module.fixed_points {
            let point = self.domains.least_fixed_point(&self.maps[name])?;
            theme.goal("least fixed point", &format!("lfp({name})"));
            theme.message("least fixed point established", AnsiColor::BrightGreen);
            theme.field(
                "result",
                &format!("lfp({name}) = {}", theme.term(module, &point.value)),
            );
            if args.config.trace {
                theme.field(
                    "ascending chain",
                    &point
                        .chain
                        .iter()
                        .map(|term| theme.term(module, term))
                        .collect::<Vec<_>>()
                        .join(&format!(" {} ", theme.order())),
                );
            }

            theme.end();
        }

        theme.conclusion(
            &format!(
                "{}; {}",
                count(
                    module.domains.len(),
                    "finite domain checked",
                    "finite domains checked"
                ),
                count(
                    self.maps.len(),
                    "semantic map checked",
                    "semantic maps checked"
                )
            ),
            AnsiColor::BrightGreen,
        );

        Ok(true)
    }

    pub fn correspond(
        &self,
        engine: &soda_semantics::Engine,
        args: &Arguments,
        context: &mut Context,
    ) -> Result<bool, CommandError> {
        let source = args
            .term
            .as_deref()
            .ok_or("correspond requires operation(map_name)")?;
        let term = context.parse_input(source)?;
        let Term::App(operation, arguments) = &term else {
            return Err("expected operation(map_name)".into());
        };

        let [Term::App(name, parameters)] = arguments.as_slice() else {
            return Err("expected operation(map_name)".into());
        };

        if !parameters.is_empty() {
            return Err("expected a semantic map name without arguments".into());
        }

        let map = self
            .maps
            .get(name)
            .ok_or_else(|| format!("unknown semantic map {name}"))?;
        let result =
            soda_proof::check_correspondence(engine, &self.domains, map, operation, &args.config)?;
        let theme = Theme::new(args);
        theme.heading("finite correspondence", &context.module);
        theme.goal(
            "correspondence obligation",
            &format!("{operation} agrees with semantic map {name}"),
        );

        match result {
            soda_proof::Correspondence::Established { cases } => {
                theme.message("finite correspondence established", AnsiColor::BrightGreen);
                theme.detail(&format!("all {cases} represented domain values checked"));

                theme.end();

                Ok(true)
            }
            soda_proof::Correspondence::Counterexample {
                input,
                expected,
                actual,
                status,
            } => {
                theme.message("counterexample found", AnsiColor::BrightRed);
                theme.field("input", &theme.term(&context.module, &input));
                theme.field("expected", &theme.term(&context.module, &expected));
                theme.field("actual", &theme.term(&context.module, &actual));
                theme.line(&theme.evaluation_status(&status));
                context.notice(
                    &term,
                    soda_diagnostics::Diagnostic::error(
                        "E4201",
                        "execution disagrees with its finite semantic map",
                    )
                    .note(format!("at {input}: expected {expected}, found {actual}"))
                    .help(
                        "compare the operational rules with the semantic map entry for this input",
                    ),
                );

                theme.end();

                Ok(false)
            }
            soda_proof::Correspondence::Unknown { input, status } => {
                theme.message("correspondence not established", AnsiColor::BrightYellow);
                theme.field("unresolved input", &theme.term(&context.module, &input));
                theme.line(&theme.evaluation_status(&status));
                let mut diagnostic = soda_diagnostics::Diagnostic::error(
                    "W4202",
                    "finite correspondence was not established",
                )
                .note(format!("evaluation stopped at {input}"))
                .help("increase the evaluation resource bound and check again");
                diagnostic.severity = soda_diagnostics::Severity::Warning;
                context.notice(&term, diagnostic);

                theme.end();

                Ok(false)
            }
        }
    }
}
