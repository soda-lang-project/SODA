use crate::{Arguments, check_input, diagnostics::Context, presentation::Theme};
use soda_ast::Module;
use soda_relations::{Relations, SearchConfig};
use std::time::Instant;
mod report;
use report::{Reporter, Summary};

pub fn run(module: &Module, args: &Arguments, context: &mut Context) -> Result<bool, String> {
    let relations = Relations::from_module(module)?;
    let queries = match &args.term {
        Some(term) => vec![context.parse_input(term)?],
        None => module.queries.clone(),
    };

    if queries.is_empty() {
        let message = format!("`infer` has no queries to run in module `{}`", module.name);
        let mut diagnostic = soda_diagnostics::Diagnostic::error("E4103", &message)
            .note("`infer` searches `query` declarations; it does not execute `eval` declarations");

        if let Some(site) = context
            .locations
            .declarations
            .iter()
            .find(|site| site.kind == soda_parser::DeclarationKind::Eval)
        {
            diagnostic = diagnostic.primary(
                context.source,
                site.span,
                "this is an `eval` declaration, executed with `run` or `denote`",
            );
        }

        if !module.evaluations.is_empty() {
            diagnostic = diagnostic
                .note(format!(
                    "this module contains {} `eval` declarations and no `query` declarations",
                    module.evaluations.len()
                ))
                .help(format!(
                    "execute these examples with: soda run \"{}\"",
                    args.file
                ));
        }
        diagnostic = diagnostic.help("to use inference, add a `query` declaration or pass a declared judgment as the term argument");

        return Err(context.error(diagnostic, message));
    }

    let config = SearchConfig {
        max_attempts: args.config.fuel,
        max_depth: args
            .config
            .max_depth
            .min(soda_relations::MAX_DERIVATION_DEPTH),
        max_answers: args.answers,
    };
    let reporter = Reporter {
        module,
        relations: &relations,
        theme: Theme::new(args),
        config: &config,
    };
    let start = Instant::now();
    let mut summary = Summary::default();
    println!("\n{}\n", reporter.heading(queries.len()));
    for (index, query) in queries.iter().enumerate() {
        check_input(
            module,
            query,
            crate::application::InputKind::Judgment,
            context,
        )?;
        let result = relations.search_checked(query, &config)?;

        for answer in &result.answers {
            relations.verify(&answer.derivation)?;
        }

        context.search(query, &result);
        summary.record(&result);
        print!(
            "{}",
            reporter.query(index + 1, queries.len(), query, &result, args.config.trace)
        );
    }

    println!("{}", reporter.summary(&summary, start.elapsed()));

    Ok(summary.without_derivation == 0 && summary.unresolved == 0)
}
