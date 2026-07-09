use crate::{Kind, ParseError, Parser, Token, lex};
use soda_ast::{Module, NotationDecl};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Hole(usize),
    Literal(Kind),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notation {
    pub judgment: String,
    pub arity: usize,
    pub parts: Vec<Part>,
}
impl Notation {
    pub(crate) fn operator(&self) -> (bool, Vec<&Kind>) {
        let infix = matches!(self.parts.first(), Some(Part::Hole(_)));
        let tokens = self
            .parts
            .iter()
            .skip(usize::from(infix))
            .take_while(|part| matches!(part, Part::Literal(_)))
            .map(|part| match part {
                Part::Literal(token) => token,
                Part::Hole(_) => unreachable!(),
            })
            .collect();

        (infix, tokens)
    }

    pub(crate) fn matches(&self, tokens: &[Token], infix: bool) -> bool {
        let (is_infix, operator) = self.operator();

        is_infix == infix
            && operator
                .iter()
                .zip(tokens)
                .all(|(expected, actual)| **expected == actual.kind)
            && tokens.len() >= operator.len()
    }

    pub(crate) fn operator_len(&self) -> usize {
        self.operator().1.len()
    }

    pub(crate) fn conflicts(&self, other: &Self) -> bool {
        if self.operator() != other.operator() {
            return false;
        }

        for (left, right) in self.parts.iter().zip(&other.parts) {
            match (left, right) {
                (Part::Hole(_), Part::Hole(_)) => {}
                (Part::Literal(a), Part::Literal(b)) if a != b => return false,
                (Part::Literal(_), Part::Literal(_)) => {}
                // Prefix and hole/literal overlaps would make parsing depend on grouping.
                _ => return true,
            }
        }

        true
    }
}

pub(crate) fn compile_notation(
    judgment: &str,
    template: &str,
    arity: usize,
) -> Result<Notation, String> {
    let tokens = lex(template).map_err(|e| format!("invalid notation template: {e}"))?;
    let mut cursor = 0;
    let mut parts = Vec::new();
    let mut holes = BTreeSet::new();

    while tokens[cursor].kind != Kind::End {
        if tokens[cursor].kind == Kind::Symbol('$') {
            cursor += 1;
            let Kind::Number(number) = &tokens[cursor].kind else {
                return Err("notation hole requires an unsigned index".into());
            };

            if !number.bytes().all(|b| b.is_ascii_digit()) {
                return Err("notation hole requires an unsigned index".into());
            }

            let index: usize = number.parse().map_err(|_| "invalid notation hole index")?;

            if index >= arity || !holes.insert(index) {
                return Err(
                    "notation holes must occur exactly once and match judgment arity".into(),
                );
            }

            if matches!(parts.last(), Some(Part::Hole(_))) {
                return Err("adjacent notation holes require a delimiter".into());
            }

            parts.push(Part::Hole(index));
        } else {
            let token = tokens[cursor].kind.clone();

            if !matches!(token, Kind::Symbol(_) | Kind::Arrow)
                || matches!(token, Kind::Symbol('(' | ')' | ',' | ';' | '?' | '@' | '.'))
            {
                return Err(
                    "notation literals must be punctuation excluding term delimiters".into(),
                );
            }

            parts.push(Part::Literal(token));
        }
        cursor += 1;
    }

    if holes.len() != arity || !parts.iter().any(|p| matches!(p, Part::Literal(_))) {
        return Err("notation needs punctuation and one hole per judgment argument".into());
    }

    let operator_index = usize::from(matches!(parts.first(), Some(Part::Hole(_))));

    if matches!(
        parts.get(operator_index),
        Some(Part::Literal(Kind::Arrow | Kind::Symbol('=')))
    ) {
        return Err("notation cannot begin with reserved equality or rule arrows".into());
    }

    Ok(Notation {
        judgment: judgment.into(),
        arity,
        parts,
    })
}

impl Parser<'_> {
    pub(super) fn add_notation(
        &mut self,
        module: &Module,
        declaration: &NotationDecl,
    ) -> Result<(), Box<ParseError>> {
        let arity = module
            .judgments
            .iter()
            .find(|j| j.name == declaration.judgment)
            .ok_or_else(|| self.fail("notation requires a previously declared judgment"))?
            .arguments
            .len();

        for template in [&declaration.unicode, &declaration.ascii] {
            let notation = compile_notation(&declaration.judgment, template, arity)
                .map_err(|m| self.fail(m))?;

            if let Some(previous) = self.notations.iter().find(|n| n.conflicts(&notation)) {
                if previous == &notation {
                    continue;
                }

                return Err(self.fail("ambiguous notation: templates overlap"));
            }

            self.notations.push(notation);
        }

        Ok(())
    }
}
