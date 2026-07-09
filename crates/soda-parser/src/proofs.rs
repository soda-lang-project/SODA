use crate::error::error;
use crate::{Kind, ParseError, Parser};
use soda_ast::{InductionCase, InductionProof, Module, ProofStep, Substitution};
use soda_diagnostics::Span;
use std::collections::BTreeSet;

impl Parser<'_> {
    pub(super) fn proof_body(&mut self) -> Result<Vec<ProofStep>, Box<ParseError>> {
        self.symbol('{')?;
        let mut steps = Vec::new();

        while !self.eat(&Kind::Symbol('}')) {
            steps.push(self.proof_step()?);
        }

        Ok(steps)
    }

    pub(super) fn proof_step(&mut self) -> Result<ProofStep, Box<ParseError>> {
        let start = self.current().offset;
        let step = self.proof_step_inner()?;

        if self.track_locations {
            self.locations
                .steps
                .push(Span::new(start, self.previous_end()));
        }

        Ok(step)
    }

    pub(super) fn proof_step_inner(&mut self) -> Result<ProofStep, Box<ParseError>> {
        let axiom = self.identifier()?;
        let reverse = self.keyword("reverse");
        let mut path = Vec::new();

        if self.keyword("at") {
            self.symbol('[')?;

            if !self.eat(&Kind::Symbol(']')) {
                loop {
                    let Kind::Number(number) = &self.current().kind else {
                        return Err(self.expected("expected a nonnegative child index"));
                    };

                    if number.starts_with('-') || number.starts_with('+') {
                        return Err(self.fail("child indices must be unsigned integers"));
                    }

                    path.push(
                        number
                            .parse()
                            .map_err(|_| self.fail("child index is too large"))?,
                    );
                    self.cursor += 1;
                    if self.eat(&Kind::Symbol(']')) {
                        break;
                    }

                    self.symbol(',')?;
                }
            }
        }

        let mut substitution = Substitution::new();

        if self.keyword("with") {
            self.symbol('{')?;

            if !self.eat(&Kind::Symbol('}')) {
                loop {
                    self.eat(&Kind::Symbol('?'));
                    let offset = self.current().offset;
                    let name = self.identifier()?;
                    self.symbol('=')?;
                    let term = self.term(0)?;

                    if substitution.insert(name.clone(), term).is_some() {
                        return Err(error(
                            self.source,
                            offset,
                            format!("duplicate substitution for ?{name}"),
                        ));
                    }

                    if self.eat(&Kind::Symbol('}')) {
                        break;
                    }

                    self.symbol(',')?;
                }
            }
        }

        self.symbol(';')?;

        Ok(ProofStep {
            axiom,
            reverse,
            path,
            substitution,
        })
    }

    pub(super) fn induction_declaration(
        &mut self,
        module: &mut Module,
        names: &mut BTreeSet<String>,
    ) -> Result<(), Box<ParseError>> {
        let name = self.unique_name(names)?;

        if !self.keyword("on") {
            return Err(self.expected("expected 'on' induction variable"));
        }

        self.symbol('?')?;
        let variable = self.identifier()?;
        self.symbol(':')?;
        let sort = self.sort()?;
        self.symbol(':')?;
        let lhs = self.term(0)?;
        self.symbol('=')?;
        let rhs = self.term(0)?;
        self.symbol('{')?;
        let mut cases = Vec::new();

        while !self.eat(&Kind::Symbol('}')) {
            let case_start = self.cursor;

            if !self.keyword("case") {
                return Err(self.expected("expected induction case"));
            }

            let constructor = self.identifier()?;
            let mut arguments = Vec::new();

            if self.eat(&Kind::Symbol('(')) && !self.eat(&Kind::Symbol(')')) {
                loop {
                    arguments.push(self.identifier()?);
                    if self.eat(&Kind::Symbol(')')) {
                        break;
                    }

                    self.symbol(',')?;
                }
            }

            let steps = self.proof_body()?;
            self.record_declaration(case_start);
            cases.push(InductionCase {
                constructor,
                arguments,
                steps,
            });
        }

        self.eat(&Kind::Symbol(';'));
        module.inductions.push(InductionProof {
            name,
            variable,
            sort,
            lhs,
            rhs,
            cases,
        });

        Ok(())
    }
}
