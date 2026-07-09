use crate::{Kind, MAX_TERM_DEPTH, ParseError, Parser};
use soda_ast::{Module, MonadDecl, RecordDecl, Signature, SignatureKind, SortDecl, Term};
use std::collections::BTreeMap;

impl Parser<'_> {
    pub(super) fn record_type_declaration(
        &mut self,
        module: &mut Module,
    ) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        self.symbol('=')?;
        let constructor = self.identifier()?;
        if self
            .records
            .iter()
            .any(|r| r.name == name || r.constructor == constructor)
            || self.signatures.iter().any(|s| s.name == constructor)
            || module.sorts.iter().any(|s| s.name == name)
        {
            return Err(self.fail(format!(
                "duplicate record sort or constructor '{name}' / '{constructor}'"
            )));
        }
        self.symbol('{')?;
        let mut fields = Vec::new();
        if !self.eat(&Kind::Symbol('}')) {
            loop {
                let field = self.identifier()?;
                if fields.iter().any(|(n, _)| n == &field) {
                    return Err(self.fail(format!("duplicate record field '{field}'")));
                }
                self.symbol(':')?;
                fields.push((field, self.sort()?));
                if self.eat(&Kind::Symbol('}')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        self.symbol(';')?;
        let record = RecordDecl {
            name: name.clone(),
            constructor: constructor.clone(),
            fields,
        };
        let signature = Signature {
            name: constructor,
            arguments: record.fields.iter().map(|(_, s)| s.clone()).collect(),
            result: name.clone(),
            kind: SignatureKind::Constructor,
        };
        module.sorts.push(SortDecl {
            name,
            inductive: true,
        });
        module.signatures.push(signature.clone());
        self.signatures.push(signature);
        self.records.push(record.clone());
        module.records.push(record);
        Ok(())
    }

    pub(super) fn monad_declaration(&mut self, module: &mut Module) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        if self.monads.iter().any(|m| m.name == name) {
            return Err(self.fail(format!("duplicate monad instance '{name}'")));
        }
        self.symbol('{')?;
        let mut operations = BTreeMap::new();
        while !self.eat(&Kind::Symbol('}')) {
            let operation = self.identifier()?;
            if operation != "bind" && operation != "return" {
                return Err(self.fail("monad operation must be 'bind' or 'return'"));
            }
            self.symbol('=')?;
            let function = self.identifier()?;
            self.symbol(';')?;
            if operations.insert(operation.clone(), function).is_some() {
                return Err(self.fail(format!("duplicate monad operation '{operation}'")));
            }
        }
        self.symbol(';')?;
        let bind = operations
            .remove("bind")
            .ok_or_else(|| self.fail("monad instance requires 'bind'"))?;
        let unit = operations
            .remove("return")
            .ok_or_else(|| self.fail("monad instance requires 'return'"))?;
        let instance = MonadDecl { name, bind, unit };
        self.monad_sort(&instance)?;
        self.monads.push(instance.clone());
        module.monads.push(instance);
        Ok(())
    }

    fn monad_sort(&self, instance: &MonadDecl) -> Result<String, Box<ParseError>> {
        let bind = self
            .signatures
            .iter()
            .find(|s| s.name == instance.bind)
            .ok_or_else(|| {
                self.fail(format!(
                    "monad '{}' bind function '{}' must be declared first",
                    instance.name, instance.bind
                ))
            })?;
        let unit = self
            .signatures
            .iter()
            .find(|s| s.name == instance.unit)
            .ok_or_else(|| {
                self.fail(format!(
                    "monad '{}' return function '{}' must be declared first",
                    instance.name, instance.unit
                ))
            })?;
        if bind.kind != SignatureKind::Function
            || unit.kind != SignatureKind::Function
            || bind.arguments.len() != 2
            || unit.arguments.len() != 1
        {
            return Err(
                self.fail("monad requires a binary bind function and unary return function")
            );
        }
        let expected = format!("Fn({},{})", unit.arguments[0], bind.result);
        if bind.arguments[0] != bind.result
            || bind.arguments[1] != expected
            || unit.result != bind.result
        {
            return Err(self.fail(format!("monad '{}' requires bind(M, Fn(A,M)): M and return(A): M; incompatible function signatures", instance.name)));
        }
        Ok(unit.arguments[0].clone())
    }

    pub(super) fn record_literal(
        &mut self,
        constructor: &str,
        depth: usize,
    ) -> Result<Term, Box<ParseError>> {
        let record = self
            .records
            .iter()
            .find(|r| r.constructor == constructor)
            .cloned()
            .ok_or_else(|| self.fail(format!("unknown record constructor '{constructor}'")))?;
        self.symbol('{')?;
        let mut values = BTreeMap::new();
        if !self.eat(&Kind::Symbol('}')) {
            loop {
                let name = self.identifier()?;
                if !record.fields.iter().any(|(n, _)| n == &name) {
                    return Err(
                        self.fail(format!("unknown field '{name}' in record '{constructor}'"))
                    );
                }
                self.symbol(':')?;
                let value = self.term(depth + 1)?;
                if values.insert(name.clone(), value).is_some() {
                    return Err(self.fail(format!(
                        "duplicate field '{name}' in record '{constructor}'"
                    )));
                }
                if self.eat(&Kind::Symbol('}')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        let mut arguments = Vec::new();
        for (name, _) in &record.fields {
            arguments.push(values.remove(name).ok_or_else(|| {
                self.fail(format!("missing field '{name}' in record '{constructor}'"))
            })?);
        }
        Ok(Term::app(constructor, arguments))
    }

    pub(super) fn list_literal(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        let mut items = Vec::new();
        let mut tail = Term::app("end", vec![]);
        if !self.eat(&Kind::Symbol(']')) {
            loop {
                if items.len() + depth >= MAX_TERM_DEPTH {
                    return Err(self.fail("list literal exceeds term nesting limit"));
                }
                items.push(self.term(depth + 1)?);
                if self.eat(&Kind::Symbol(']')) {
                    break;
                }
                if self.eat(&Kind::Symbol('|')) {
                    tail = self.term(depth + 1)?;
                    self.symbol(']')?;
                    break;
                }
                self.symbol(',')?;
            }
        }
        for item in items.into_iter().rev() {
            tail = Term::app("cons", vec![item, tail]);
        }
        Ok(tail)
    }

    pub(super) fn do_block(&mut self, depth: usize) -> Result<Term, Box<ParseError>> {
        let name = self.identifier()?;
        let instance = self
            .monads
            .iter()
            .find(|m| m.name == name)
            .cloned()
            .ok_or_else(|| self.fail(format!("unknown monad instance '{name}'")))?;
        let sort = self.monad_sort(&instance)?;
        self.symbol('{')?;
        let saved_names = self.do_names.len();
        let result = self.do_statements(&instance, &sort, depth);
        self.do_names.truncate(saved_names);
        result
    }

    fn do_statements(
        &mut self,
        instance: &MonadDecl,
        sort: &str,
        depth: usize,
    ) -> Result<Term, Box<ParseError>> {
        if depth >= MAX_TERM_DEPTH {
            return Err(self.fail("do block exceeds term nesting limit"));
        }
        if self.keyword("return") {
            let value = self.term(depth + 1)?;
            self.eat(&Kind::Symbol(';'));
            self.symbol('}')?;
            return Ok(Term::app(&instance.unit, vec![value]));
        }
        if self.current().kind == Kind::Symbol('}') {
            return Err(self.fail("do block requires a final computation or return value"));
        }
        let binding = matches!(&self.current().kind, Kind::Ident(_))
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|t| t.kind == Kind::Symbol(':') || t.kind == Kind::Symbol('<'));
        let parameter = if binding {
            self.identifier()?
        } else {
            let mut generated = format!("__soda_do_{}", self.cursor);
            while self.source.contains(&generated) {
                generated.push('_');
            }
            generated
        };
        let parameter_sort = if binding && self.eat(&Kind::Symbol(':')) {
            self.sort()?
        } else {
            sort.to_owned()
        };
        if parameter_sort != sort {
            return Err(self.fail(format!(
                "do binding '{parameter}' has sort {parameter_sort}, but monad '{}' binds {sort}",
                instance.name
            )));
        }
        if binding {
            self.symbol('<')?;
            self.symbol('-')?;
        }
        let computation = self.term(depth + 1)?;
        if !binding && self.eat(&Kind::Symbol('}')) {
            return Ok(computation);
        }
        self.symbol(';')?;
        if !binding && self.eat(&Kind::Symbol('}')) {
            return Ok(computation);
        }
        if binding {
            self.do_names.push(parameter.clone());
        }
        let body = self.do_statements(instance, sort, depth + 1)?;
        if binding {
            self.do_names.pop();
        }
        Ok(Term::app(
            &instance.bind,
            vec![
                computation,
                Term::Binder {
                    parameter,
                    sort: parameter_sort,
                    body: Box::new(body),
                },
            ],
        ))
    }
}
