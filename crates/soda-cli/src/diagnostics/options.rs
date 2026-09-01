use clap::{CommandFactory, ValueEnum};
use soda_diagnostics::{Diagnostic, RenderOptions, Report, Span, Style, Suggestion, nearest};
use std::env;
use std::io::{self, IsTerminal};

/// Terminal policy is resolved separately for stdout and stderr.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    fn enabled(self, terminal: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => {
                terminal
                    && env::var_os("NO_COLOR").is_none()
                    && env::var("TERM").as_deref() != Ok("dumb")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum DiagnosticFormat {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub render: RenderOptions,
    pub format: DiagnosticFormat,
    pub stdout_color: bool,
}

impl Options {
    pub fn from_args(args: &[String]) -> Self {
        let args = args
            .split(|argument| argument == "--")
            .next()
            .unwrap_or_default();
        let value = |flag: &str| {
            args.iter().enumerate().rev().find_map(|(index, argument)| {
                if argument == flag {
                    args.get(index + 1).map(String::as_str)
                } else {
                    argument
                        .split_once('=')
                        .filter(|(key, _)| *key == flag)
                        .map(|(_, value)| value)
                }
            })
        };

        Self::new(
            value("--color")
                .and_then(|value| ColorMode::from_str(value, false).ok())
                .unwrap_or_default(),
            if value("--diagnostic-style") == Some("unicode") {
                Style::Unicode
            } else {
                Style::Ascii
            },
            value("--diagnostic-format")
                .and_then(|value| DiagnosticFormat::from_str(value, false).ok())
                .unwrap_or_default(),
            value("--diagnostic-width").and_then(|value| value.parse().ok()),
        )
    }

    pub fn new(
        color: ColorMode,
        style: Style,
        format: DiagnosticFormat,
        width: Option<usize>,
    ) -> Self {
        Self {
            render: RenderOptions {
                color: color.enabled(io::stderr().is_terminal()),
                width: width
                    .or_else(|| env::var("COLUMNS").ok()?.parse().ok())
                    .unwrap_or(soda_diagnostics::DEFAULT_RENDER_WIDTH)
                    .clamp(
                        soda_diagnostics::MIN_RENDER_WIDTH,
                        soda_diagnostics::MAX_RENDER_WIDTH,
                    ),
                style,
                ..RenderOptions::default()
            },
            format,
            stdout_color: format != DiagnosticFormat::Json
                && color.enabled(io::stdout().is_terminal()),
        }
    }

    pub fn print(&self, report: &Report) {
        if self.format == DiagnosticFormat::Json {
            eprint!("{}", report.to_json());
        } else if !report.diagnostics.is_empty() {
            eprint!("{}", report.render(&self.render));
        }
    }
}

pub fn argument_error(message: String, args: &[String]) -> Report {
    let mut report = Report::default();
    let text = args.join(" ");
    let source = report.sources.add("<command line>", &text);
    let mut diagnostic = Diagnostic::error("E0100", "invalid command line")
        .note(&message)
        .help("run `soda --help` for commands and options");

    if let Some(token) = message
        .split_whitespace()
        .find(|token| token.starts_with("--"))
        && let Some(start) = text.find(token)
    {
        diagnostic = diagnostic.primary(source, Span::new(start, start + token.len()), &message);

        let cli = crate::cli::Cli::command();
        let flags: Vec<_> = cli
            .get_arguments()
            .filter_map(|argument| argument.get_long())
            .map(|long| format!("--{long}"))
            .collect();

        if let Some(suggestion) = nearest(token, flags.iter().map(String::as_str)) {
            diagnostic.suggestions.push(Suggestion {
                source,
                span: Span::new(start, start + token.len()),
                replacement: suggestion.into(),
                message: format!("did you mean `{suggestion}`?"),
            });
        }
    }

    if let Some(command) = message
        .strip_prefix("unknown command ")
        .and_then(|message| message.split(';').next())
    {
        diagnostic = diagnostic.primary(source, Span::new(0, command.len()), "unknown command");

        let cli = crate::cli::Cli::command();

        if let Some(candidate) = nearest(
            command,
            cli.get_subcommands().map(|command| command.get_name()),
        ) {
            diagnostic.suggestions.push(Suggestion {
                source,
                span: Span::new(0, command.len()),
                replacement: candidate.into(),
                message: format!("did you mean `{candidate}`?"),
            });
        }
    }

    report.diagnostics.push(diagnostic);

    report
}
