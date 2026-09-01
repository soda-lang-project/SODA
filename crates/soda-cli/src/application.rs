use crate::diagnostics::Context;
use crate::{checking, domains};
use soda_ast::Module;
use soda_parser::NotationStyle;
use soda_semantics::{Config, Engine};

const MAX_REPORTED_SORT_ERRORS: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommandKind {
    Check,

    Prove,

}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputKind {
    Term,
    Judgment,
}

pub(crate) struct Arguments {
    pub command: CommandKind,
    pub file: String,
    pub term: Option<String>,
    pub config: Config,
    pub states: usize,
    pub transitions: usize,
    pub answers: usize,
    pub notation: NotationStyle,
    pub output_color: bool,
}

pub(crate) struct Execution {
    pub success: bool,
    pub report: soda_diagnostics::Report,
}

pub(crate) fn execute(args: Arguments) -> Result<Execution, soda_diagnostics::Report> {
    let mut context = Context::read(&args.file)?;
    let module = context.module.clone();

    if let Err(errors) = soda_sorts::validate_module_all(&module) {
        context.report.diagnostics = errors
            .iter()
            .take(MAX_REPORTED_SORT_ERRORS)
            .map(|error| context.sort_error(error))
            .collect();
        context.report.diagnostics.sort_by_key(|diagnostic| {
            diagnostic
                .labels
                .first()
                .map(|label| (label.source, label.span.start))
        });
        if errors.len() > MAX_REPORTED_SORT_ERRORS {
            context.report.diagnostics.push(
                soda_diagnostics::Diagnostic::error(
                    "E1099",
                    format!(
                        "{} further sort errors were omitted",
                        errors.len() - MAX_REPORTED_SORT_ERRORS
                    ),
                )
                .help("fix the reported definitions and run the checker again"),
            );
        }

        return Err(context.report);
    }

    match execute_checked(&module, &args, &mut context) {
        Ok(success) => Ok(Execution {
            success,
            report: context.report,
        }),
        Err(message) => Err(context.finish_error(message)),
    }
}

fn execute_checked(
    module: &Module,
    args: &Arguments,
    context: &mut Context,
) -> Result<bool, crate::error::CommandError> {
    soda_verifier::verify_module(module).map_err(|error| {
        let diagnostic = context.verification(&error);

        context.error(diagnostic, error.to_string())
    })?;
    let theorems = soda_proof::verify_inductions(module)?;
    let domains = domains::CheckedDomains::new(module)?;
    let engine = Engine::new(module.rules.clone())?;

    let result: Result<bool, String> = match args.command {
        CommandKind::Check | CommandKind::Prove => {
            checking::show(module, args, &theorems);

            Ok(true)
        }

    };

    result.map_err(Into::into)
}

pub(crate) fn check_input(
    module: &Module,
    term: &soda_ast::Term,
    kind: InputKind,
    context: &mut Context,
) -> Result<(), String> {
    let checker = soda_sorts::TypeChecker::new(module).map_err(|error| error.to_string())?;

    if checker.is_typed() || soda_sorts::requires_sort_check(term) {
        let result = if kind == InputKind::Judgment {
            checker.check_judgment(term)
        } else {
            checker.check_term(term).map(|_| ())
        };

        result.map_err(|error| {
            let diagnostic = context.sort_error(&error);

            context.error(diagnostic, error.to_string())
        })?;
    }

    Ok(())
}
