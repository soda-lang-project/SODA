use crate::{Kind, Notation, Parser, Part};
use soda_ast::{Module, Term};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotationStyle {
    #[default]
    Canonical,
    Unicode,
    Ascii,
}

/// Groups nested notation; invalid metadata falls back to canonical applications.
pub fn format_term_in(module: &Module, term: &Term, style: NotationStyle) -> String {
    if style == NotationStyle::Canonical {
        return term.to_string();
    }

    let mut parser = Parser::new("").expect("empty source lexes");

    for declaration in &module.notations {
        if parser.add_notation(module, declaration).is_err() {
            return term.to_string();
        }
    }

    let mut notations = Vec::new();

    for declaration in &module.notations {
        let template = match style {
            NotationStyle::Unicode => &declaration.unicode,
            NotationStyle::Ascii => &declaration.ascii,
            NotationStyle::Canonical => unreachable!(),
        };

        let Some(arity) = module
            .judgments
            .iter()
            .find(|j| j.name == declaration.judgment)
            .map(|j| j.arguments.len())
        else {
            return term.to_string();
        };

        let Ok(compiled) =
            crate::notation::compile_notation(&declaration.judgment, template, arity)
        else {
            return term.to_string();
        };

        notations.push(compiled);
    }

    Renderer { notations }.term(term, false)
}

struct Renderer {
    notations: Vec<Notation>,
}

impl Renderer {
    fn term(&self, term: &Term, nested: bool) -> String {
        match term {
            Term::App(name, args) => {
                if let Some(notation) = self
                    .notations
                    .iter()
                    .find(|n| n.judgment == *name && n.arity == args.len())
                {
                    let text = self.notated(notation, args);

                    return if nested { format!("({text})") } else { text };
                }

                if args.is_empty() {
                    return name.clone();
                }

                let arguments = args
                    .iter()
                    .map(|arg| self.term(arg, true))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!("{name}({arguments})")
            }
            Term::Binder {
                parameter,
                sort,
                body,
            } => format!("bind({parameter}: {sort}, {})", self.term(body, true)),
            _ => term.to_string(),
        }
    }

    fn notated(&self, notation: &Notation, args: &[Term]) -> String {
        let mut pieces = Vec::new();
        let mut literal = String::new();

        for part in &notation.parts {
            match part {
                Part::Hole(index) => {
                    if !literal.is_empty() {
                        pieces.push(std::mem::take(&mut literal));
                    }

                    pieces.push(self.term(&args[*index], true));
                }
                Part::Literal(Kind::Symbol(ch)) => {
                    if (*ch == '>' && literal.ends_with('='))
                        || (*ch == '/' && literal.ends_with('/'))
                    {
                        literal.push(' ');
                    }

                    literal.push(*ch);
                }
                Part::Literal(Kind::Arrow) => literal.push_str("=>"),
                Part::Literal(_) => unreachable!("compiled notation only contains punctuation"),
            }
        }

        if !literal.is_empty() {
            pieces.push(literal);
        }

        pieces.join(" ")
    }
}
