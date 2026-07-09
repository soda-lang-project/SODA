use crate::{Kind, ParseError, ParseErrorKind, Parser};
use soda_ast::{Clause, ClauseDetail, ComputedBinding, Equation, Module, Proof, Rule, Term};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DefinitionKind {
    Rule,
    Axiom,
    Proof,
    Infer,
}

impl Parser<'_> {
    pub(super) fn definition(
        &mut self,
        module: &mut Module,
        names: &mut BTreeSet<String>,
        kind: DefinitionKind,
    ) -> Result<(), Box<ParseError>> {
        let name = self.unique_name(names)?;
        self.symbol(':')?;
        let lhs = self.term(0)?;

        match kind {
            DefinitionKind::Infer => self.clause(module, name, lhs),
            DefinitionKind::Rule => {
                module.rules.push(self.rewrite_body(name, lhs)?);
                Ok(())
            }
            DefinitionKind::Axiom | DefinitionKind::Proof => {
                self.symbol('=')?;
                let rhs = self.term(0)?;

                if kind == DefinitionKind::Axiom {
                    self.symbol(';')?;
                    module.equations.push(Equation { name, lhs, rhs });
                } else {
                    let steps = self.proof_body()?;
                    self.eat(&Kind::Symbol(';'));
                    module.proofs.push(Proof {
                        name,
                        lhs,
                        rhs,
                        steps,
                    });
                }

                Ok(())
            }
        }
    }

    pub(super) fn rewrite_body(
        &mut self,
        name: String,
        lhs: Term,
    ) -> Result<Rule, Box<ParseError>> {
        if !self.eat(&Kind::Arrow) {
            return Err(self
                .fail("expected '=>'")
                .with_kind(ParseErrorKind::ExpectedToken("=>".into())));
        }
        let rhs = self.term(0)?;
        let condition = if self.keyword("when") {
            Some(self.term(0)?)
        } else {
            None
        };
        self.symbol(';')?;
        Ok(Rule {
            name,
            lhs,
            rhs,
            condition,
        })
    }

    fn clause(
        &mut self,
        module: &mut Module,
        name: String,
        conclusion: Term,
    ) -> Result<(), Box<ParseError>> {
        let mut premises = Vec::new();
        let mut premise_labels = Vec::new();
        let mut bindings = Vec::new();

        if self.keyword("if") {
            loop {
                if self.keyword("let") {
                    let pattern = self.primary(0)?;
                    self.symbol('=')?;
                    let value = self.term(0)?;
                    bindings.push(ComputedBinding {
                        before_premise: premises.len(),
                        pattern,
                        value,
                    });
                    if self.eat(&Kind::Symbol(',')) {
                        continue;
                    }
                    break;
                }
                let label = if matches!(self.current().kind, Kind::Ident(_))
                    && self
                        .tokens
                        .get(self.cursor + 1)
                        .is_some_and(|token| token.kind == Kind::Symbol(':'))
                {
                    let label = self.identifier()?;
                    self.symbol(':')?;

                    Some(label)
                } else {
                    None
                };

                premise_labels.push(label);
                premises.push(self.term(0)?);

                if !self.eat(&Kind::Symbol(',')) {
                    break;
                }
            }
        }

        while self.keyword("let") {
            let pattern = self.primary(0)?;
            self.symbol('=')?;
            let value = self.term(0)?;
            bindings.push(ComputedBinding {
                before_premise: premises.len(),
                pattern,
                value,
            });
            self.eat(&Kind::Symbol(','));
        }
        let mut conditions = Vec::new();

        if self.keyword("when") {
            loop {
                conditions.push(self.term(0)?);

                if !self.eat(&Kind::Symbol(',')) {
                    break;
                }
            }
        }

        self.symbol(';')?;
        if premise_labels.iter().any(Option::is_some)
            || !conditions.is_empty()
            || !bindings.is_empty()
        {
            module.clause_details.push(ClauseDetail {
                clause: name.clone(),
                premise_labels,
                bindings,
                conditions,
            });
        }

        module.clauses.push(Clause {
            name,
            conclusion,
            premises,
        });

        Ok(())
    }
}
