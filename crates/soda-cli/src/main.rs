use clap::{CommandFactory, Parser};
use std::env;
use std::process::ExitCode;

mod application;
mod checking;
mod cli;
mod diagnostics;
mod domains;
mod error;

mod presentation;
use application::{Arguments, check_input, execute};

const NOT_ESTABLISHED: u8 = 1;
const INVALID_INPUT: u8 = 2;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let diagnostics = diagnostics::Options::from_args(&args);
    let mut command = cli::Cli::command().color(if diagnostics.render.color {
        clap::ColorChoice::Always
    } else {
        clap::ColorChoice::Never
    });

    if args.is_empty() {
        let _ = command.print_help();
        println!();

        return ExitCode::SUCCESS;
    }

    let parsed =
        match cli::Cli::try_parse_from(std::iter::once("soda".to_owned()).chain(args.clone())) {
            Ok(parsed) => parsed,
            Err(error) if error.exit_code() == 0 => {
                let _ = error.with_cmd(&command).print();

                return ExitCode::SUCCESS;
            }
            Err(error) => {
                if diagnostics.format == diagnostics::DiagnosticFormat::Json {
                    diagnostics.print(&diagnostics::argument_error(
                        cli::error_message(&error),
                        &args,
                    ));
                } else {
                    let _ = error.with_cmd(&command).print();
                }

                return ExitCode::from(INVALID_INPUT);
            }
        };

    let diagnostics = parsed.diagnostics();

    if let Some(result) = parsed.run_tooling() {
        return match result {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(NOT_ESTABLISHED),
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(INVALID_INPUT)
            }
        };
    }

    match execute(parsed.into_arguments()) {
        Ok(execution) => {
            diagnostics.print(&execution.report);

            if execution.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(NOT_ESTABLISHED)
            }
        }
        Err(report) => {
            diagnostics.print(&report);

            ExitCode::from(INVALID_INPUT)
        }
    }
}
