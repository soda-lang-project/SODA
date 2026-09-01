use crate::presentation::{Theme, count};
use anstyle::{AnsiColor, Style};
use soda_ast::{Module, Term};
use soda_parser::NotationStyle;
use soda_relations::{Derivation, Relations, SearchConfig, SearchResult};
use std::fmt::Write;
use std::time::Duration;

#[derive(Default)]
pub(super) struct Summary {
    pub(super) resolved: usize,
    pub(super) without_derivation: usize,
    pub(super) unresolved: usize,
    pub(super) derivations: usize,
    pub(super) attempts: usize,
    pub(super) incomplete: usize,
}

impl Summary {
    pub(super) fn record(&mut self, result: &SearchResult) {
        if !result.answers.is_empty() {
            self.resolved += 1;
        } else if result.complete {
            self.without_derivation += 1;
        } else {
            self.unresolved += 1;
        }
        self.derivations += result.answers.len();
        self.attempts += result.attempts;
        self.incomplete += usize::from(!result.complete);
    }
}

pub(super) struct Reporter<'a> {
    pub(super) module: &'a Module,
    pub(super) relations: &'a Relations,
    pub(super) theme: Theme,
    pub(super) config: &'a SearchConfig,
}

impl Reporter<'_> {
    pub(super) fn term(&self, term: &Term) -> String {
        self.theme.term(self.module, term)
    }

    pub(super) fn heading(&self, total: usize) -> String {
        format!(
            "{} in {}",
            self.theme
                .paint(&count(total, "goal", "goals"), AnsiColor::BrightCyan, true),
            self.theme.styled(&self.module.name, Style::new().bold())
        )
    }

    pub(super) fn query(
        &self,
        index: usize,
        total: usize,
        query: &Term,
        result: &SearchResult,
        trace: bool,
    ) -> String {
        let mut output = format!("{}\n\n", self.theme.emphasis(&self.term(query)));

        if result.answers.is_empty() {
            let (message, tone) = if result.complete {
                ("no derivation found", AnsiColor::BrightRed)
            } else if result.attempts >= self.config.max_attempts
                || result.answers.len() >= self.config.max_answers
            {
                (
                    "search limit reached; goal remains unresolved",
                    AnsiColor::BrightYellow,
                )
            } else {
                (
                    "search incomplete; goal remains unresolved",
                    AnsiColor::BrightYellow,
                )
            };

            writeln!(output, "  {}", self.theme.paint(message, tone, true)).unwrap();
        }

        for (index, answer) in result.answers.iter().enumerate() {
            let title = if result.answers.len() == 1 {
                "derivation checked".into()
            } else {
                format!("derivation {}/{} checked", index + 1, result.answers.len())
            };

            writeln!(
                output,
                "  {}",
                self.theme.paint(&title, AnsiColor::BrightGreen, true)
            )
            .unwrap();

            if trace {
                self.derivation(&mut output, &answer.derivation, "    ", true, None);
            } else if &answer.term != query || result.answers.len() > 1 {
                writeln!(output, "    {}", self.term(&answer.term)).unwrap();
            }
        }

        let completeness = if result.complete {
            self.theme.paint("complete search", AnsiColor::White, false)
        } else {
            self.theme
                .paint("incomplete search", AnsiColor::BrightYellow, false)
        };

        writeln!(
            output,
            "  {}, {completeness} ({})\n",
            count(result.answers.len(), "answer", "answers"),
            count(result.attempts, "attempt", "attempts")
        )
        .unwrap();

        self.theme.panel(&format!("goal {index}/{total}"), &output)
    }

    pub(super) fn derivation(
        &self,
        output: &mut String,
        proof: &Derivation,
        prefix: &str,
        last: bool,
        label: Option<&str>,
    ) {
        let unicode = self.theme.notation == NotationStyle::Unicode;
        let branch = match (unicode, last) {
            (true, true) => "└─ ",
            (true, false) => "├─ ",
            (false, true) => "`- ",
            (false, false) => "+- ",
        };

        let label = label
            .map(|label| {
                format!(
                    "{} ",
                    self.theme
                        .paint(&format!("{label}:"), AnsiColor::BrightCyan, true)
                )
            })
            .unwrap_or_default();
        writeln!(
            output,
            "{prefix}{}{label}{}  {}",
            self.theme.paint(branch, AnsiColor::White, false),
            self.term(&proof.conclusion),
            self.theme.paint(
                &format!("by {}", proof.rule),
                AnsiColor::BrightMagenta,
                false
            )
        )
        .unwrap();
        let prefix = format!(
            "{prefix}{}",
            if last {
                "   "
            } else if unicode {
                "│  "
            } else {
                "|  "
            }
        );

        for (index, premise) in proof.premises.iter().enumerate() {
            let label = self
                .relations
                .clause_detail(&proof.rule)
                .and_then(|detail| detail.premise_labels.get(index))
                .and_then(Option::as_deref);
            self.derivation(
                output,
                premise,
                &prefix,
                index + 1 == proof.premises.len(),
                label,
            );
        }
    }

    pub(super) fn summary(&self, summary: &Summary, elapsed: Duration) -> String {
        let mut output = format!(
            "{}\n",
            self.theme.paint(
                &count(
                    summary.derivations,
                    "derivation checked",
                    "derivations checked"
                ),
                if summary.derivations > 0 {
                    AnsiColor::BrightGreen
                } else {
                    AnsiColor::White
                },
                true
            )
        );
        let mut goals = vec![count(summary.resolved, "goal resolved", "goals resolved")];

        if summary.without_derivation > 0 {
            goals.push(self.theme.paint(
                &count(
                    summary.without_derivation,
                    "goal without a derivation",
                    "goals without a derivation",
                ),
                AnsiColor::BrightRed,
                false,
            ));
        }

        if summary.unresolved > 0 {
            goals.push(self.theme.paint(
                &count(summary.unresolved, "unresolved goal", "unresolved goals"),
                AnsiColor::BrightYellow,
                false,
            ));
        }

        writeln!(
            output,
            "{}; {}; finished in {:.2}s",
            goals.join("; "),
            count(summary.attempts, "attempt", "attempts"),
            elapsed.as_secs_f64()
        )
        .unwrap();
        if summary.incomplete > 0 {
            writeln!(
                output,
                "{}",
                self.theme.paint(
                    &format!(
                        "{}; additional answers may remain undiscovered",
                        count(
                            summary.incomplete,
                            "incomplete search",
                            "incomplete searches"
                        )
                    ),
                    AnsiColor::BrightYellow,
                    false
                )
            )
            .unwrap();
        }

        output
    }
}
