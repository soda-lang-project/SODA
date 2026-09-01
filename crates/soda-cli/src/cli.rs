use crate::diagnostics::{ColorMode, DiagnosticFormat};
use clap::{Args, Parser, Subcommand};
use soda_semantics::Config;
mod values;
use values::{DrawingStyle, Notation, ReductionOrder};

const DEFAULT_EXPLORED_STATES: usize = 1_000;
const DEFAULT_TRANSITION_DEPTH: usize = 64;

use crate::{Arguments, application::CommandKind};

#[derive(Parser)]
#[command(
    name = "soda",
    styles = command_styles(),
    version,
    about = "SODA — Semantics: Operational, Denotational, Axiomatic",
    after_help = "Terms containing spaces must be shell-quoted. '-' reads source from stdin.\nCompare checks examples, not universal equivalence. Proofs are relative to axioms.",
    subcommand_required = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    options: Options,
}

#[derive(Subcommand)]
enum Command {

    /// Validate syntax, rules, axioms, and every proof certificate.
    Check(Source),
    /// Operationally evaluate a term or the file's eval declarations.
    #[command(visible_alias = "eval")]
    Run(Input),
    /// Compositionally evaluate a term or the file's eval declarations.
    Denote(Input),
    /// Check finite domains, apply semantic maps, and compute fixed points.
    Domains(Input),
    /// Check operation(map_name) against every represented domain input.
    Correspond(Input),
    /// Compare both evaluators on the specified concrete inputs.
    Compare(Input),
    /// Check every explicit equational proof certificate.
    Prove(Source),
    /// Search inference rules for query declarations or a query term.
    Infer(Input),
    /// Explore all contextual rewrites, including overlapping rules.
    Explore(Input),
    /// Report syntactic termination criterion and critical pairs.
    Analyze(Source),

}

#[derive(Args)]
struct Source {
    /// Source filename, or - to read stdin.
    file: String,
}

#[derive(Args)]
struct Input {
    #[command(flatten)]
    source: Source,
    /// Override the file's input declarations with this term.
    #[arg(allow_negative_numbers = true)]
    term: Option<String>,
}

#[derive(Args)]
struct Options {
    /// Maximum reductions or inference attempts.
    #[arg(long, global = true, default_value_t = soda_semantics::DEFAULT_FUEL)]
    fuel: usize,
    /// Maximum term/evaluation depth.
    #[arg(long, global = true, default_value_t = soda_semantics::MAX_DEPTH, value_parser = depth)]
    depth: usize,
    /// Operational reduction order.
    #[arg(long, global = true, value_enum, default_value = "outermost")]
    strategy: ReductionOrder,
    /// Print reduction or derivation traces.
    #[arg(long, global = true)]
    trace: bool,
    /// Maximum explored states.
    #[arg(long, global = true, default_value_t = DEFAULT_EXPLORED_STATES)]
    states: usize,
    /// Maximum path length in exploration.
    #[arg(long, global = true, default_value_t = DEFAULT_TRANSITION_DEPTH)]
    transitions: usize,
    /// Maximum answers per inference subgoal.
    #[arg(long, global = true, default_value_t = soda_relations::DEFAULT_MAX_ANSWERS)]
    answers: usize,
    /// Select result notation.
    #[arg(long, global = true, value_enum, default_value = "canonical")]
    notation: Notation,
    /// Colour for all human reports and diagnostics (auto respects NO_COLOR).
    #[arg(long, global = true, value_enum, default_value = "auto")]
    color: ColorMode,
    /// Diagnostic drawing characters.
    #[arg(long, global = true, value_enum, default_value = "ascii")]
    diagnostic_style: DrawingStyle,
    /// Structured diagnostics on stderr.
    #[arg(long, global = true, value_enum, default_value = "human")]
    diagnostic_format: DiagnosticFormat,
    /// Diagnostic width, 40–240 columns (default COLUMNS or 100).
    #[arg(long, global = true, value_parser = width)]
    diagnostic_width: Option<usize>,
}

fn depth(value: &str) -> Result<usize, String> {
    bounded(value, 1, soda_semantics::MAX_DEPTH)
}

fn width(value: &str) -> Result<usize, String> {
    bounded(
        value,
        soda_diagnostics::MIN_RENDER_WIDTH,
        soda_diagnostics::MAX_RENDER_WIDTH,
    )
}

fn bounded(value: &str, minimum: usize, maximum: usize) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| (minimum..=maximum).contains(value))
        .ok_or_else(|| format!("expected an integer between {minimum} and {maximum}"))
}

impl Cli {
    pub(crate) fn run_tooling(&self) -> Option<Result<bool, String>> {
        match &self.command {

            _ => None,
        }
    }
    pub(crate) fn diagnostics(&self) -> crate::diagnostics::Options {
        crate::diagnostics::Options::new(
            self.options.color,
            self.options.diagnostic_style.into(),
            self.options.diagnostic_format,
            self.options.diagnostic_width,
        )
    }

    pub(crate) fn into_arguments(self) -> Arguments {
        let output_color = self.diagnostics().stdout_color;
        let (command, source, term) = match self.command {

            Command::Check(source) => (CommandKind::Check, source, None),
            Command::Prove(source) => (CommandKind::Prove, source, None),
            Command::Analyze(source) => (CommandKind::Analyze, source, None),

            Command::Run(input) => (CommandKind::Run, input.source, input.term),
            Command::Denote(input) => (CommandKind::Denote, input.source, input.term),
            Command::Domains(input) => (CommandKind::Domains, input.source, input.term),
            Command::Correspond(input) => (CommandKind::Correspond, input.source, input.term),
            Command::Compare(input) => (CommandKind::Compare, input.source, input.term),
            Command::Infer(input) => (CommandKind::Infer, input.source, input.term),
            Command::Explore(input) => (CommandKind::Explore, input.source, input.term),
        };

        Arguments {
            output_color,
            command,
            file: source.file,
            term,
            config: Config {
                fuel: self.options.fuel,
                max_depth: self.options.depth,
                trace: self.options.trace,
                strategy: self.options.strategy.into(),
            },
            states: self.options.states,
            transitions: self.options.transitions,
            answers: self.options.answers,
            notation: self.options.notation.into(),
        }
    }
}

pub(crate) fn error_message(error: &clap::Error) -> String {
    use clap::error::{ContextKind, ErrorKind};

    match error.kind() {
        ErrorKind::UnknownArgument => error
            .get(ContextKind::InvalidArg)
            .map(|argument| format!("unknown option {argument}")),
        ErrorKind::InvalidSubcommand => error
            .get(ContextKind::InvalidSubcommand)
            .map(|command| format!("unknown command {command}; see --help")),
        _ => None,
    }
    .unwrap_or_else(|| error.to_string())
}

fn command_styles() -> clap::builder::Styles {
    use anstyle::{AnsiColor, Style};
    let bright = |color: AnsiColor| Style::new().fg_color(Some(color.into()));

    clap::builder::Styles::styled()
        .header(bright(AnsiColor::BrightCyan).bold())
        .usage(bright(AnsiColor::BrightCyan).bold())
        .literal(Style::new().bold())
        .placeholder(bright(AnsiColor::White))
        .error(bright(AnsiColor::BrightRed).bold())
        .valid(bright(AnsiColor::BrightGreen))
        .invalid(bright(AnsiColor::BrightRed))
}

