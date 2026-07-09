use crate::{DeclarationKind, Kind, ParseError, Parser};
use soda_ast::{Module, Signature, SignatureKind, Term};
use std::collections::BTreeSet;

impl Parser<'_> {
    pub(super) fn function_declaration(
        &mut self,
        module: &mut Module,
        names: &mut BTreeSet<String>,
    ) -> Result<(), Box<ParseError>> {
        let name = self.identifier()?;
        self.symbol('(')?;
        let mut arguments = Vec::new();
        let mut parameters = Vec::new();
        let mut parameter_names = BTreeSet::new();
        if !self.eat(&Kind::Symbol(')')) {
            loop {
                let parameter = if self.eat(&Kind::Symbol('?')) {
                    let parameter = self.identifier()?;
                    if !parameter_names.insert(parameter.clone()) {
                        return Err(
                            self.fail(format!("duplicate function parameter '?{parameter}'"))
                        );
                    }
                    self.symbol(':')?;
                    Some(parameter)
                } else {
                    None
                };
                parameters.push(parameter);
                arguments.push(self.sort()?);
                if self.eat(&Kind::Symbol(')')) {
                    break;
                }
                self.symbol(',')?;
            }
        }
        self.symbol(':')?;
        let result = self.sort()?;
        let arity = arguments.len();
        let signature = Signature {
            name: name.clone(),
            arguments,
            result,
            kind: SignatureKind::Function,
        };
        self.signatures.push(signature.clone());
        module.signatures.push(signature);
        if self.current().kind == Kind::Arrow {
            let Some(parameters) = parameters.into_iter().collect::<Option<Vec<_>>>() else {
                return Err(self.fail(
                    "compact function definitions require named parameters, such as '?x: Int'",
                ));
            };
            if !names.insert(name.clone()) {
                return Err(self.fail(format!("duplicate definition '{name}'")));
            }
            let lhs = Term::app(&name, parameters.into_iter().map(Term::Var).collect());
            module.rules.push(self.rewrite_body(name, lhs)?);
            return Ok(());
        }
        if parameters.iter().any(Option::is_some) {
            return Err(
                self.expected("named function parameters require a compact '=>' definition")
            );
        }
        if self.eat(&Kind::Symbol(';')) {
            return Ok(());
        }
        self.symbol('{')?;
        while !self.eat(&Kind::Symbol('}')) {
            let start = self.cursor;
            if !self.keyword("case") {
                return Err(self.expected("expected 'case' or '}' in function definition"));
            }
            let case = self.unique_name(names)?;
            self.symbol(':')?;
            self.symbol('(')?;
            let mut patterns = Vec::new();
            if !self.eat(&Kind::Symbol(')')) {
                loop {
                    patterns.push(self.term(0)?);
                    if self.eat(&Kind::Symbol(')')) {
                        break;
                    }
                    self.symbol(',')?;
                }
            }
            if patterns.len() != arity {
                return Err(self.fail(format!("case '{case}' of function '{name}' expects {arity} argument patterns, found {}", patterns.len())));
            }
            module
                .rules
                .push(self.rewrite_body(case, Term::app(&name, patterns))?);
            self.record_declaration(start);
            if self.track_locations {
                self.locations.declarations.last_mut().unwrap().kind = DeclarationKind::Rule;
            }
        }
        self.eat(&Kind::Symbol(';'));
        Ok(())
    }
}
