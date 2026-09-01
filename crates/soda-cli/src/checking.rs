use crate::{
    Arguments,
    presentation::{Theme, count},
};
use anstyle::AnsiColor;
use soda_ast::{Module, ProofStep};
use soda_proof::CheckedTheorem;

pub fn show(module: &Module, args: &Arguments, theorems: &[CheckedTheorem]) {
    let theme = Theme::new(args);
    theme.heading(
        if args.command == crate::application::CommandKind::Prove {
            "proof checking"
        } else {
            "definition checking"
        },
        module,
    );
    theme.goal("module declarations", &module.name);
    theme.message("module checked", AnsiColor::BrightGreen);
    theme.detail(&format!(
        "{} rules, {} inference rules, {} axioms, {} checked proofs",
        module.rules.len(),
        module.clauses.len(),
        module.equations.len(),
        module.proofs.len()
    ));
    if !module.sorts.is_empty() || !module.judgments.is_empty() {
        theme.detail(&format!(
            "{} sorts, {} signatures, {} declared judgments",
            module.sorts.len(),
            module.signatures.len(),
            module.judgments.len()
        ));
    }

    theme.end();
    for proof in &module.proofs {
        theme.goal(
            &format!("theorem {}", proof.name),
            &format!(
                "{} = {}",
                theme.term(module, &proof.lhs),
                theme.term(module, &proof.rhs)
            ),
        );
        theme.message("certificate checked", AnsiColor::BrightGreen);
        theme.detail(&count(
            proof.steps.len(),
            "equational step",
            "equational steps",
        ));
        if args.config.trace {
            steps(&theme, module, &proof.steps);
        }

        theme.end();
    }

    for theorem in theorems {
        if let Some(source) = module
            .inductions
            .iter()
            .find(|proof| proof.name == theorem.name)
        {
            theme.goal(
                &format!(
                    "theorem {}: {} checked constructor cases",
                    theorem.name, theorem.cases
                ),
                &format!(
                    "{} = {}",
                    theme.term(module, &source.lhs),
                    theme.term(module, &source.rhs)
                ),
            );
            theme.message("induction checked", AnsiColor::BrightGreen);
            theme.detail(&count(theorem.steps, "equational step", "equational steps"));
            if args.config.trace {
                for case in &source.cases {
                    theme.message(&format!("case {}", case.constructor), AnsiColor::BrightCyan);
                    steps(&theme, module, &case.steps);
                }
            }

            theme.end();
        }
    }

    theme.conclusion(
        &format!(
            "{}; {}",
            count(
                module.proofs.len(),
                "certificate checked",
                "certificates checked"
            ),
            count(theorems.len(), "induction checked", "inductions checked")
        ),
        AnsiColor::BrightGreen,
    );

    if !module.equations.is_empty() {
        theme.note("axioms are assumptions; certificates are checked relative to them");
    }
}

fn steps(theme: &Theme, module: &Module, steps: &[ProofStep]) {
    for (index, step) in steps.iter().enumerate() {
        theme.line(&format!(
            "  {}  {}{} at {:?}",
            theme.paint(&format!("step {}", index + 1), AnsiColor::BrightCyan, false),
            theme.paint(
                &format!("by {}", step.axiom),
                AnsiColor::BrightMagenta,
                false
            ),
            if step.reverse { " reverse" } else { "" },
            step.path
        ));

        for (name, value) in &step.substitution {
            theme.line(&format!(
                "    ?{} := {}",
                soda_diagnostics::escape_terminal(name),
                theme.term(module, value)
            ));
        }
    }
}
