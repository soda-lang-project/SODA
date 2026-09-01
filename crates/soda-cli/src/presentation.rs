use crate::Arguments;
use anstyle::{AnsiColor, Style};
use soda_ast::{Module, Term};
use soda_diagnostics::escape_terminal;
use soda_parser::{NotationStyle, format_term_in};
use soda_semantics::{Limit, Status};
use std::fmt::Write;

pub struct Theme {
    color: bool,
    pub notation: NotationStyle,
}

impl Theme {
    pub fn new(args: &Arguments) -> Self {
        Self {
            color: args.output_color,
            notation: args.notation,
        }
    }

    pub fn styled(&self, text: &str, style: Style) -> String {
        let text = escape_terminal(text);

        if !self.color {
            return text;
        }

        format!("{}{text}{}", style.render(), style.render_reset())
    }

    pub fn paint(&self, text: &str, color: AnsiColor, bold: bool) -> String {
        let mut style = Style::new().fg_color(Some(color.into()));

        if bold {
            style = style.bold();
        }

        self.styled(text, style)
    }

    pub fn emphasis(&self, text: &str) -> String {
        self.styled(text, Style::new().bold())
    }

    pub fn term(&self, module: &Module, term: &Term) -> String {
        escape_terminal(&format_term_in(module, term, self.notation))
    }

    pub fn heading(&self, topic: &str, module: &Module) {
        println!(
            "\n{} in {}\n",
            self.paint(topic, AnsiColor::BrightCyan, true),
            self.emphasis(&module.name)
        );
    }

    fn frame(&self) -> (&'static str, &'static str, &'static str) {
        if self.notation == NotationStyle::Unicode {
            ("╭─", "│", "╰─")
        } else {
            ("+--", "|", "`--")
        }
    }

    fn edge(&self, text: &str) -> String {
        self.paint(text, AnsiColor::BrightCyan, false)
    }

    fn opening(&self, label: &str) -> String {
        let (top, bar, _) = self.frame();

        format!(
            "{} {}\n{}\n",
            self.edge(top),
            self.paint(label, AnsiColor::BrightCyan, true),
            self.edge(bar)
        )
    }

    pub fn panel(&self, label: &str, body: &str) -> String {
        let mut output = self.opening(label);

        for line in body.trim_end_matches('\n').split('\n') {
            writeln!(
                output,
                "{}{}",
                self.edge(self.frame().1),
                if line.is_empty() {
                    String::new()
                } else {
                    format!(" {line}")
                }
            )
            .unwrap();
        }

        writeln!(output, "{}\n", self.edge(self.frame().2)).unwrap();

        output
    }

    pub fn goal(&self, label: &str, target: &str) {
        print!("{}", self.opening(label));
        self.line(&self.emphasis(target));
        self.line("");
    }

    pub fn line(&self, text: &str) {
        for line in text.split('\n') {
            if line.is_empty() {
                println!("{}", self.edge(self.frame().1));
            } else {
                println!("{} {line}", self.edge(self.frame().1));
            }
        }
    }

    pub fn end(&self) {
        println!("{}\n", self.edge(self.frame().2));
    }

    pub fn message(&self, text: &str, color: AnsiColor) {
        self.line(&self.paint(text, color, true));
    }

    pub fn detail(&self, text: &str) {
        self.line(&self.paint(text, AnsiColor::White, false));
    }

    pub fn field(&self, label: &str, value: &str) {
        self.line(&format!(
            "{} {}",
            self.paint(&format!("{label}:"), AnsiColor::BrightCyan, false),
            self.emphasis(value)
        ));
    }

    pub fn note(&self, text: &str) {
        println!(
            "{} {}",
            self.paint("note:", AnsiColor::BrightCyan, true),
            self.paint(text, AnsiColor::White, false)
        );
    }

    pub fn conclusion(&self, text: &str, color: AnsiColor) {
        println!("{}\n", self.paint(text, color, true));
    }

    pub fn arrow(&self) -> &'static str {
        if self.notation == NotationStyle::Unicode {
            "→"
        } else {
            "=>"
        }
    }

    pub fn order(&self) -> &'static str {
        if self.notation == NotationStyle::Unicode {
            "⊑"
        } else {
            "<="
        }
    }

    pub fn evaluation_status(&self, status: &Status) -> String {
        let (text, color) = match status {
            Status::NormalForm => ("normal form reached", AnsiColor::BrightGreen),
            Status::Limit(Limit::Fuel) => ("reduction limit reached", AnsiColor::BrightYellow),
            Status::Limit(Limit::Depth) => ("depth limit reached", AnsiColor::BrightYellow),
            Status::Stuck(_) => ("stuck computation", AnsiColor::BrightRed),
            Status::Error(_) => ("evaluation error", AnsiColor::BrightRed),
        };

        self.paint(text, color, true)
    }
}

pub fn count(value: usize, singular: &str, plural: &str) -> String {
    format!("{value} {}", if value == 1 { singular } else { plural })
}
