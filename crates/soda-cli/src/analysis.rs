use crate::{
    Arguments,
    presentation::{Theme, count},
};
use anstyle::AnsiColor;
use soda_ast::Module;

pub fn run(module: &Module, args: &Arguments) -> Result<bool, String> {
    let theme = Theme::new(args);
    theme.heading("syntactic analysis", module);
    theme.goal(
        "termination obligation",
        "strictly decreasing rewrite size without variable duplication",
    );
    let termination = soda_verifier::analyze_termination(&module.rules);
    theme.message(
        if termination.proven {
            "termination proved by decreasing size"
        } else {
            "termination not established"
        },
        if termination.proven {
            AnsiColor::BrightGreen
        } else {
            AnsiColor::BrightYellow
        },
    );
    if !termination.unproven_rules.is_empty() {
        theme.field(
            "rules outside the criterion",
            &termination.unproven_rules.join(", "),
        );
    }

    theme.end();
    theme.goal("critical pairs", "overlapping syntactic rewrite rules");
    let analysis =
        soda_verifier::critical_pairs(&module.rules).map_err(|error| error.to_string())?;
    theme.message(
        &count(
            analysis.pairs.len(),
            "nontrivial syntactic critical pair",
            "nontrivial syntactic critical pairs",
        ),
        if analysis.pairs.is_empty() {
            AnsiColor::BrightCyan
        } else {
            AnsiColor::BrightYellow
        },
    );
    for (index, pair) in analysis.pairs.iter().enumerate() {
        theme.line(&format!(
            "  {}  {} / {} at {:?}",
            theme.paint(
                &format!("overlap {}", index + 1),
                AnsiColor::BrightCyan,
                true
            ),
            theme.paint(&pair.outer_rule, AnsiColor::BrightMagenta, false),
            theme.paint(&pair.inner_rule, AnsiColor::BrightMagenta, false),
            pair.position
        ));
        theme.field("source", &theme.term(module, &pair.source));
        theme.field("outer reduct", &theme.term(module, &pair.outer_reduct));
        theme.field("inner reduct", &theme.term(module, &pair.inner_reduct));
    }

    theme.message("analysis scope", AnsiColor::BrightCyan);
    for limitation in analysis.limitations {
        theme.detail(&limitation);
    }

    theme.end();
    theme.conclusion("syntactic analysis complete", AnsiColor::BrightCyan);

    Ok(true)
}
