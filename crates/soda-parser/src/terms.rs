use crate::{Kind, MAX_TERM_DEPTH, Notation, ParseError, Parser, Part};
use soda_ast::Term;

impl Parser<'_> {
    pub(super) fn term(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        let start = self.current().offset;
        let term = self.term_inner(depth)?;
        self.record_term(start, &term);

        Ok(term)
    }

    pub(super) fn term_inner(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        let first = self.primary(depth)?;

        Ok(self.notated(Some(&first), depth + 1)?.unwrap_or(first))
    }

    pub(super) fn notated(
        &mut self,
        first: Option<&Term>,
        depth: usize,
    ) -> Result<Option<Term>, Box<ParseError>> {
        let mut candidates: Vec<_> = self
            .notations
            .iter()
            .filter(|n| n.matches(&self.tokens[self.cursor..], first.is_some()))
            .cloned()
            .collect();
        let Some(longest) = candidates.iter().map(Notation::operator_len).max() else {
            return Ok(None);
        };

        candidates.retain(|n| n.operator_len() == longest);
        let mut values: Vec<_> = first.cloned().into_iter().collect();
        let mut position = usize::from(first.is_some());

        // Candidates share argument structure until their first differing literal.

        while let Some(part) = candidates[0].parts.get(position) {
            match part {
                Part::Hole(_) => values.push(self.primary(depth + 1)?),
                Part::Literal(_) => {
                    let token = &self.current().kind;
                    candidates.retain(
                        |n| matches!(n.parts.get(position), Some(Part::Literal(t)) if t == token),
                    );
                    if candidates.is_empty() {
                        return Err(self.expected("expected notation punctuation"));
                    }

                    self.cursor += 1;
                }
            }
            position += 1;
        }

        let notation = &candidates[0];
        let mut arguments = vec![None; notation.arity];
        let mut values = values.into_iter();

        for part in &notation.parts {
            if let Part::Hole(index) = part {
                arguments[*index] = values.next();
            }
        }

        Ok(Some(Term::app(
            &notation.judgment,
            arguments
                .into_iter()
                .map(|arg| arg.expect("validated holes"))
                .collect(),
        )))
    }

    pub(super) fn primary(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        let start = self.current().offset;
        let term = self.primary_inner(depth)?;
        self.record_term(start, &term);

        Ok(term)
    }

    pub(super) fn primary_inner(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        if depth >= MAX_TERM_DEPTH {
            return Err(self
                .fail(format!("term nesting exceeds {MAX_TERM_DEPTH} levels"))
                .with_kind(crate::ParseErrorKind::NestingLimit));
        }

        if let Some(term) = self.notated(None, depth + 1)? {
            return Ok(term);
        }

        if self.keyword("do") {
            return self.do_block(depth + 1);
        }
        if self.conditional_ahead() {
            self.cursor += 1;
            let condition = self.term(depth + 1)?;
            if !self.keyword("then") {
                return Err(self.expected("expected 'then' after if condition"));
            }
            let yes = self.term(depth + 1)?;
            if !self.keyword("else") {
                return Err(self.expected("expected 'else' after then branch"));
            }
            let no = self.term(depth + 1)?;
            return Ok(Term::app("if", vec![condition, yes, no]));
        }
        if self.eat(&Kind::Symbol('[')) {
            return self.list_literal(depth + 1);
        }

        if self.eat(&Kind::Symbol('(')) {
            let term = self.term(depth + 1)?;
            self.symbol(')')?;

            return Ok(term);
        }

        if self.eat(&Kind::Symbol('?')) {
            return Ok(Term::Var(self.identifier()?));
        }

        if self.eat(&Kind::Symbol('⊥')) {
            return Ok(Term::app("bottom", vec![]));
        }

        if self.eat(&Kind::Symbol('@')) {
            return Ok(Term::Name(self.identifier()?));
        }

        let ascii_lambda =
            self.current().kind == Kind::Ident("lambda".into()) && self.binder_header_ahead(1);

        if ascii_lambda || self.current().kind == Kind::Symbol('λ') {
            self.cursor += 1;
            let (parameter, sort) = self.binder_header()?;
            self.symbol('.')?;

            return Ok(Term::Binder {
                parameter,
                sort,
                body: Box::new(self.term(depth + 1)?),
            });
        }

        match self.current().kind.clone() {
            Kind::Number(number) => {
                let value = number.parse().map_err(|_| {
                    self.fail("integer is outside the signed 64-bit range")
                        .with_kind(crate::ParseErrorKind::IntegerRange)
                })?;
                self.cursor += 1;

                Ok(Term::Int(value))
            }
            Kind::Ident(name) => {
                self.cursor += 1;
                if name == "true" || name == "false" {
                    return Ok(Term::Bool(name == "true"));
                }

                if name == "bind"
                    && self.current().kind == Kind::Symbol('(')
                    && self.binder_header_ahead(1)
                {
                    self.cursor += 1;
                    let (parameter, sort) = self.binder_header()?;
                    self.symbol(',')?;
                    let body = Box::new(self.term(depth + 1)?);
                    self.symbol(')')?;

                    return Ok(Term::Binder {
                        parameter,
                        sort,
                        body,
                    });
                }

                if self.current().kind == Kind::Symbol('{')
                    && self.records.iter().any(|r| r.constructor == name)
                {
                    return self.record_literal(&name, depth + 1);
                }
                if self.do_names.contains(&name) && self.current().kind != Kind::Symbol('(') {
                    return Ok(Term::Name(name));
                }

                let mut args = Vec::new();

                if self.eat(&Kind::Symbol('(')) && !self.eat(&Kind::Symbol(')')) {
                    loop {
                        args.push(self.term(depth + 1)?);
                        if self.eat(&Kind::Symbol(')')) {
                            break;
                        }

                        self.symbol(',')?;
                    }
                }

                Ok(Term::App(name, args))
            }
            _ => {
                Err(self
                    .expected("expected a term (variable, name, binder, literal, or constructor)"))
            }
        }
    }

    pub(super) fn binder_header_ahead(&self, offset: usize) -> bool {
        self.tokens
            .get(self.cursor + offset)
            .is_some_and(|token| matches!(token.kind, Kind::Ident(_)))
            && self
                .tokens
                .get(self.cursor + offset + 1)
                .is_some_and(|token| token.kind == Kind::Symbol(':'))
    }

    fn conditional_ahead(&self) -> bool {
        if self.current().kind != Kind::Ident("if".into()) {
            return false;
        }
        let Some(next) = self.tokens.get(self.cursor + 1) else {
            return false;
        };
        if next.kind != Kind::Symbol('(') {
            return !matches!(
                next.kind,
                Kind::End | Kind::Symbol(';' | ',' | ')' | ']' | '}') | Kind::Arrow
            );
        }

        let mut depth = 0usize;
        for (index, token) in self.tokens.iter().enumerate().skip(self.cursor + 1) {
            match token.kind {
                Kind::Symbol('(') => depth += 1,
                Kind::Symbol(')') => {
                    depth -= 1;
                    if depth == 0 {
                        return self
                            .tokens
                            .get(index + 1)
                            .is_some_and(|t| t.kind == Kind::Ident("then".into()));
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(super) fn binder_header(&mut self) -> Result<(String, String), Box<ParseError>> {
        let parameter = self.identifier()?;
        self.symbol(':')?;

        Ok((parameter, self.sort()?))
    }
}
