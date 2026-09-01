use crate::{
    Arguments, check_input,
    diagnostics::Context,
    presentation::{Theme, count},
};
use anstyle::AnsiColor;
use soda_ast::Module;
use soda_model::StateStatus;
use soda_semantics::Engine;

pub fn run(
    engine: &Engine,
    module: &Module,
    args: &Arguments,
    context: &mut Context,
) -> Result<bool, String> {
    let inputs = match &args.term {
        Some(term) => vec![context.parse_input(term)?],
        None => module.evaluations.clone(),
    };

    if inputs.is_empty() {
        return Err("no initial states: add an eval declaration or pass a term".into());
    }

    let config = soda_model::Config {
        max_states: args.states,
        max_depth: args.transitions,
        evaluation: args.config.clone(),
    };
    let theme = Theme::new(args);
    theme.heading("state exploration", module);
    let mut complete = 0;

    for (index, input) in inputs.iter().enumerate() {
        check_input(module, input, crate::application::InputKind::Term, context)?;
        if !input.variables().is_empty() {
            return Err(context.ground_error(input));
        }

        theme.goal(
            &format!("initial state {}/{}", index + 1, inputs.len()),
            &theme.term(module, input),
        );
        let graph = soda_model::explore(engine, input, &config);
        theme.message(
            if graph.is_complete() {
                "complete exploration"
            } else {
                "incomplete exploration"
            },
            if graph.is_complete() {
                AnsiColor::BrightGreen
            } else {
                AnsiColor::BrightYellow
            },
        );
        theme.detail(&format!(
            "{}; {}",
            count(graph.states.len(), "state", "states"),
            count(graph.transitions.len(), "transition", "transitions")
        ));
        if args.config.trace {
            for (index, state) in graph.states.iter().enumerate() {
                let (label, color) = match state.status {
                    StateStatus::Expanded => ("expanded", AnsiColor::BrightCyan),
                    StateStatus::Terminal => ("terminal", AnsiColor::BrightGreen),
                    StateStatus::Incomplete => ("incomplete", AnsiColor::BrightYellow),
                };

                theme.line(&format!(
                    "  {} {}  {}",
                    theme.paint(&format!("state {index}"), AnsiColor::BrightCyan, true),
                    theme.term(module, &state.term),
                    theme.paint(label, color, false)
                ));
            }

            for edge in &graph.transitions {
                theme.line(&format!(
                    "  [{}] {} [{}]  {} at {:?}",
                    edge.from,
                    theme.paint(theme.arrow(), AnsiColor::BrightCyan, true),
                    edge.to,
                    theme.paint(
                        &format!("by {}", edge.rule),
                        AnsiColor::BrightMagenta,
                        false
                    ),
                    edge.path
                ));
            }
        }

        context.exploration(input, &graph);
        complete += usize::from(graph.is_complete());
        theme.end();
    }

    theme.conclusion(
        &format!(
            "{}; {}",
            count(complete, "complete graph", "complete graphs"),
            count(inputs.len() - complete, "partial graph", "partial graphs")
        ),
        if complete == inputs.len() {
            AnsiColor::BrightGreen
        } else {
            AnsiColor::BrightYellow
        },
    );

    Ok(complete == inputs.len())
}
