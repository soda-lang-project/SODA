use crate::{
    Arguments, check_input,
    diagnostics::Context,
    presentation::{Theme, count},
};
use anstyle::AnsiColor;
use soda_ast::{Module, Term};
use soda_semantics::{Engine, Evaluation, Status};

enum Outcome {
    Established,
    Counterexample,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Operational,
    Compositional,
    Compare,
}

impl Mode {
    fn topic(self) -> &'static str {
        match self {
            Self::Operational => "operational evaluation",
            Self::Compositional => "compositional evaluation",
            Self::Compare => "concrete comparisons",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Operational => "run",
            Self::Compositional => "denote",
            Self::Compare => "compare",
        }
    }
}

pub fn run(
    engine: &Engine,
    module: &Module,
    args: &Arguments,
    context: &mut Context,
    mode: Mode,
) -> Result<bool, String> {
    let terms = match &args.term {
        Some(term) => vec![context.parse_input(term)?],
        None => module.evaluations.clone(),
    };

    if terms.is_empty() {
        return Err("no input terms: add an eval declaration or pass a term".into());
    }

    let theme = Theme::new(args);
    let comparison = mode == Mode::Compare;
    theme.heading(mode.topic(), module);
    let mut established = 0;
    let mut counterexamples = 0;

    for (index, term) in terms.iter().enumerate() {
        check_input(module, term, crate::application::InputKind::Term, context)?;
        if !term.variables().is_empty() {
            return Err(context.ground_error(term));
        }

        theme.goal(
            &format!(
                "{} {}/{}",
                if comparison {
                    "comparison"
                } else {
                    "evaluation"
                },
                index + 1,
                terms.len()
            ),
            &theme.term(module, term),
        );
        let complete = if comparison {
            compare(engine, module, args, context, &theme, term)
        } else {
            let result = if mode == Mode::Compositional {
                engine.denote(term, &args.config)
            } else {
                engine.evaluate(term, &args.config)
            };

            show(&theme, &result, module, args.config.trace, "derivation");
            context.runtime(term, &result, mode.label());

            if result.status == Status::NormalForm {
                Outcome::Established
            } else {
                Outcome::Unresolved
            }
        };

        match complete {
            Outcome::Established => established += 1,
            Outcome::Counterexample => counterexamples += 1,
            Outcome::Unresolved => {}
        }

        theme.end();
    }

    let noun = if comparison {
        ("comparison established", "comparisons established")
    } else {
        ("normal form established", "normal forms established")
    };

    let remaining = terms.len() - established - counterexamples;
    let mut summary = count(established, noun.0, noun.1);

    if counterexamples > 0 {
        summary.push_str(&format!(
            "; {}",
            count(counterexamples, "counterexample", "counterexamples")
        ));
    }

    if remaining > 0 {
        summary.push_str(&format!(
            "; {}",
            count(remaining, "obligation unresolved", "obligations unresolved")
        ));
    }

    theme.conclusion(
        &summary,
        if counterexamples > 0 {
            AnsiColor::BrightRed
        } else if remaining == 0 {
            AnsiColor::BrightGreen
        } else {
            AnsiColor::BrightYellow
        },
    );
    if comparison {
        theme.note("agreement concerns these concrete inputs only");
    }

    Ok(established == terms.len())
}

fn compare(
    engine: &Engine,
    module: &Module,
    args: &Arguments,
    context: &mut Context,
    theme: &Theme,
    term: &Term,
) -> Outcome {
    let operational = engine.evaluate(term, &args.config);
    let denotational = engine.denote(term, &args.config);
    theme.message("operational interpretation", AnsiColor::BrightCyan);
    show(
        theme,
        &operational,
        module,
        args.config.trace,
        "operational derivation",
    );
    theme.message("denotational interpretation", AnsiColor::BrightCyan);
    show(
        theme,
        &denotational,
        module,
        args.config.trace,
        "denotational derivation",
    );
    context.runtime(term, &operational, "operational");
    context.runtime(term, &denotational, "compositional");
    let normal =
        operational.status == Status::NormalForm && denotational.status == Status::NormalForm;
    let agrees = normal && operational.term.alpha_equivalent(&denotational.term);

    if agrees {
        theme.message("evaluators agree on this input", AnsiColor::BrightGreen);
    } else if normal {
        theme.message(
            "different normal forms; agreement not established",
            AnsiColor::BrightRed,
        );
        context.notice(term, soda_diagnostics::Diagnostic::error("E4401", "the evaluators produced different normal forms")
            .note(format!("operational:   {}", operational.term)).note(format!("compositional: {}", denotational.term))
            .help("inspect overlapping rules and strategy-sensitive reductions; use `--trace` to compare their steps"));
    } else {
        theme.message("agreement not established", AnsiColor::BrightYellow);
    }

    if agrees {
        Outcome::Established
    } else if normal {
        Outcome::Counterexample
    } else {
        Outcome::Unresolved
    }
}

fn show(theme: &Theme, evaluation: &Evaluation, module: &Module, trace: bool, label: &str) {
    if trace && !evaluation.trace.is_empty() {
        theme.message(label, AnsiColor::BrightCyan);

        for (index, step) in evaluation.trace.iter().enumerate() {
            theme.line(&format!(
                "  {}  {} at {:?}",
                theme.paint(&format!("step {}", index + 1), AnsiColor::BrightCyan, false),
                theme.paint(
                    &format!("by {}", step.rule),
                    AnsiColor::BrightMagenta,
                    false
                ),
                step.path
            ));
            theme.line(&format!(
                "    {}\n    {} {}",
                theme.term(module, &step.before),
                theme.paint(theme.arrow(), AnsiColor::BrightCyan, true),
                theme.term(module, &step.after)
            ));
        }
    }

    theme.line(&theme.evaluation_status(&evaluation.status));
    theme.field("result", &theme.term(module, &evaluation.term));
    theme.detail(&count(evaluation.steps, "reduction", "reductions"));
}
