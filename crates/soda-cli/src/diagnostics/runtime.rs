use super::Context;
use soda_ast::Term;
use soda_diagnostics::{Diagnostic, Severity};
use soda_semantics::{Evaluation, Limit, RuntimeErrorKind, Status};

impl Context {
    pub fn runtime(&mut self, input: &Term, result: &Evaluation, label: &str) {
        if result.status == Status::NormalForm {
            return;
        }

        let (source, span) = self.locate(Some(input), None);
        let mut diagnostic = match &result.status {
            Status::Limit(limit) => {
                let mut diagnostic = Diagnostic::error("W4002", "evaluation stopped at its resource limit");
                diagnostic.severity = Severity::Warning;
                diagnostic.help.push(match limit { Limit::Fuel => "increase `--fuel N` to allow more reductions", Limit::Depth => "increase `--depth N` within the supported ceiling, or simplify this term" }.into());

                diagnostic.note("a resource limit does not establish divergence or a semantic counterexample")
            }
            Status::Error(error) => {
                let mut diagnostic = Diagnostic::error("E4001", "evaluation failed").note(error.to_string());

                if error.kind == RuntimeErrorKind::DivisionByZero { diagnostic.help.push("the divisor must be nonzero; guard the operation or select a safe branch with `if`".into()); }

                if error.kind == RuntimeErrorKind::Overflow { diagnostic.help.push("integer arithmetic is checked over signed 64-bit values".into()); }

                diagnostic
            }
            Status::Stuck(message) => Diagnostic::error("E4003", "no semantic rule can finish this computation").note(message).help("check the term's constructor shape, rule patterns, and guard conditions; use `--trace` to inspect reductions"),
            Status::NormalForm => unreachable!(),
        };

        diagnostic =
            diagnostic.primary(source, span, format!("while evaluating this {label} input"));
        diagnostic.notes.push(format!(
            "stopped after {} {} at `{}`",
            result.steps,
            if result.steps == 1 {
                "reduction"
            } else {
                "reductions"
            },
            result.term
        ));
        if let Some(name) = &result.last_rule
            && let Some((rule_source, site)) = self.declaration_site(name)
        {
            diagnostic = diagnostic.secondary(rule_source, site.name_span, "last applied rule");
        }

        self.report.diagnostics.push(diagnostic);
    }

    pub fn notice(&mut self, term: &Term, mut diagnostic: Diagnostic) {
        let (source, span) = self.locate(Some(term), None);
        diagnostic.labels.insert(
            0,
            soda_diagnostics::Label {
                source,
                span,
                message: "for this input".into(),
                primary: true,
            },
        );
        self.report.diagnostics.push(diagnostic);
    }

    pub fn search(&mut self, query: &Term, result: &soda_relations::SearchResult) {
        if !result.complete {
            let mut diagnostic = Diagnostic::error("W4101", "inference search is incomplete")
                .note(format!("found {} answers after {} search attempts", result.answers.len(), result.attempts))
                .help("increase `--fuel`, `--depth`, or `--answers` to continue exploring alternatives");
            diagnostic.severity = Severity::Warning;
            self.notice(query, diagnostic);
        } else if result.answers.is_empty() {
            self.notice(query, Diagnostic::error("E4102", "no derivation was found for this judgment")
                .note("search exhausted its branches without a successful rule instance")
                .help("check the declared inference rules, their premises, and their Boolean side conditions"));
        }
    }

    pub fn exploration(&mut self, input: &Term, graph: &soda_model::Exploration) {
        if graph.is_complete() {
            return;
        }

        let mut diagnostic = Diagnostic::error("W4301", "state exploration is incomplete")
            .note(format!(
                "discovered {} states and {} transitions",
                graph.states.len(),
                graph.transitions.len()
            ))
            .note("a partial graph cannot establish safety or unreachability");
        diagnostic.severity = Severity::Warning;
        for reason in &graph.incompleteness {
            match reason {
                soda_model::Incomplete::StateLimit { .. } => diagnostic
                    .help
                    .push("increase `--states N` to store more states".into()),
                soda_model::Incomplete::DepthLimit { .. } => diagnostic
                    .help
                    .push("increase `--transitions N` to explore longer paths".into()),
                soda_model::Incomplete::Enumeration { status, .. } => diagnostic
                    .notes
                    .push(format!("successor enumeration stopped: {status:?}")),
                soda_model::Incomplete::InvalidTransition { .. } => diagnostic.notes.push(
                    "a transition could not be reconstructed from its recorded position".into(),
                ),
            }
        }

        diagnostic.help.sort();
        diagnostic.help.dedup();
        self.notice(input, diagnostic);
    }
}
