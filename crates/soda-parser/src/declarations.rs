use crate::{Kind, ParseError, Parser};
use soda_ast::{DomainDecl, DomainExpr, SemanticMap};
use soda_ast::{JudgmentDecl, JudgmentKind, Module, NotationDecl};

impl Parser<'_> {
    pub(super) fn domain_declaration(
        &mut self,
        module: &mut Module,
    ) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        self.symbol('=')?;
        let operation = self.identifier()?;
        self.symbol('(')?;
        let expression = match operation.as_str() {
            "flat" => {
                let mut elements = Vec::new();

                if !self.eat(&Kind::Symbol(')')) {
                    loop {
                        elements.push(self.term(0)?);
                        if self.eat(&Kind::Symbol(')')) {
                            break;
                        }

                        self.symbol(',')?;
                    }
                }

                DomainExpr::Flat(elements)
            }
            "product" | "functions" => {
                let left = self.identifier()?;
                self.symbol(',')?;
                let right = self.identifier()?;
                self.symbol(')')?;

                if operation == "product" {
                    DomainExpr::Product(left, right)
                } else {
                    DomainExpr::Function(left, right)
                }
            }
            "lift" => {
                let inner = self.identifier()?;
                self.symbol(')')?;

                DomainExpr::Lifted(inner)
            }
            _ => return Err(self.expected("expected flat, product, functions, or lift domain")),
        };

        self.symbol(';')?;
        module.domains.push(DomainDecl { name, expression });

        Ok(())
    }

    pub(super) fn map_declaration(&mut self, module: &mut Module) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        self.symbol(':')?;
        let input = self.identifier()?;
        self.symbol('-')?;
        self.symbol('>')?;
        let output = self.identifier()?;
        self.symbol('{')?;
        let mut entries = Vec::new();

        while !self.eat(&Kind::Symbol('}')) {
            let key = self.term(0)?;

            if !self.eat(&Kind::Arrow) {
                return Err(self.expected("expected '=>' in semantic map"));
            }

            let value = self.term(0)?;
            self.symbol(';')?;
            entries.push((key, value));
        }

        self.eat(&Kind::Symbol(';'));
        module.semantic_maps.push(SemanticMap {
            name,
            input,
            output,
            entries,
        });

        Ok(())
    }

    pub(super) fn judgment_declaration(
        &mut self,
        module: &mut Module,
    ) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        let arguments = self.sorts()?;
        let kind = if self.keyword("kind") {
            match self.identifier()?.as_str() {
                "relation" => JudgmentKind::Relation,
                "transition" => JudgmentKind::Transition,
                "evaluation" => JudgmentKind::Evaluation,
                "typing" => JudgmentKind::Typing,
                "denotation" => JudgmentKind::Denotation,
                "hoare" => JudgmentKind::Hoare,
                "equality" => JudgmentKind::Equality,
                "satisfaction" => JudgmentKind::Satisfaction,
                _ => return Err(self.fail("unknown judgment kind")),
            }
        } else {
            JudgmentKind::Relation
        };

        self.symbol(';')?;
        module.judgments.push(JudgmentDecl {
            name,
            arguments,
            kind,
        });

        Ok(())
    }

    pub(super) fn notation_declaration(
        &mut self,
        module: &mut Module,
    ) -> Result<(), Box<ParseError>> {
        let judgment = self.identifier()?;
        let unicode = self.text()?;

        if !self.keyword("ascii") {
            return Err(self.expected("expected 'ascii' notation equivalent"));
        }

        let ascii = self.text()?;
        self.symbol(';')?;
        let declaration = NotationDecl {
            judgment,
            unicode,
            ascii,
        };
        self.add_notation(module, &declaration)?;
        module.notations.push(declaration);

        Ok(())
    }
}
