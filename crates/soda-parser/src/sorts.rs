use crate::{Kind, MAX_TERM_DEPTH, ParseError, Parser};
use soda_ast::{Module, Signature, SignatureKind, SortDecl};

impl Parser<'_> {
    pub(super) fn sort_declaration(
        &mut self,
        module: &mut Module,
        inductive: bool,
    ) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        module.sorts.push(SortDecl {
            name: name.clone(),
            inductive,
        });
        if !inductive || !self.eat(&Kind::Symbol('{')) {
            return self.symbol(';');
        }
        while !self.eat(&Kind::Symbol('}')) {
            let start = self.cursor;
            if !self.keyword("constructor") {
                return Err(self.expected("expected 'constructor' or '}' in datatype declaration"));
            }
            let constructor = self.identifier()?;
            let arguments = self.sorts()?;
            if self.eat(&Kind::Symbol(':')) && self.sort()? != name {
                return Err(self.fail(format!(
                    "constructor '{constructor}' must return enclosing datatype '{name}'"
                )));
            }
            self.symbol(';')?;
            let signature = Signature {
                name: constructor,
                arguments,
                result: name.clone(),
                kind: SignatureKind::Constructor,
            };
            self.signatures.push(signature.clone());
            module.signatures.push(signature);
            self.record_declaration(start);
        }
        self.eat(&Kind::Symbol(';'));
        Ok(())
    }

    pub(super) fn sort(&mut self) -> Result<String, Box<ParseError>> {
        self.sort_at(0)
    }

    pub(super) fn sort_at(&mut self, depth: usize) -> Result<String, Box<ParseError>> {
        if depth >= MAX_TERM_DEPTH {
            return Err(self
                .fail(format!("sort nesting exceeds {MAX_TERM_DEPTH} levels"))
                .with_kind(crate::ParseErrorKind::NestingLimit));
        }

        let mut name = if self.eat(&Kind::Symbol('(')) {
            let inner = self.sort_at(depth + 1)?;
            self.symbol(')')?;
            inner
        } else {
            self.identifier()?
        };

        if self.eat(&Kind::Symbol('(')) {
            let mut arguments = vec![self.sort_at(depth + 1)?];

            while self.eat(&Kind::Symbol(',')) {
                arguments.push(self.sort_at(depth + 1)?);
            }

            self.symbol(')')?;
            name = format!("{}({})", name, arguments.join(","));
        }

        if self.eat(&Kind::Symbol('-')) {
            self.symbol('>')?;
            name = format!("Fn({},{})", name, self.sort_at(depth + 1)?);
        }

        Ok(name)
    }

    pub(super) fn sorts(&mut self) -> Result<Vec<String>, Box<ParseError>> {
        self.symbol('(')?;
        let mut result = Vec::new();

        if !self.eat(&Kind::Symbol(')')) {
            loop {
                result.push(self.sort()?);
                if self.eat(&Kind::Symbol(')')) {
                    break;
                }

                self.symbol(',')?;
            }
        }

        Ok(result)
    }
}
