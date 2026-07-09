use crate::{DeclarationKind, Kind, ParseError, ParseErrorKind, Parser};
use soda_ast::{Module, Signature, SignatureKind};
use std::collections::BTreeSet;

impl Parser<'_> {
    pub(super) fn module(&mut self) -> Result<Module, Box<ParseError>> {
        if !self.keyword("module") {
            return Err(self.expected("expected 'module' declaration"));
        }

        let mut module = std::mem::take(&mut self.imported);
        module.name = self.identifier()?;
        self.symbol(';')?;
        let mut names: BTreeSet<_> = module
            .rules
            .iter()
            .map(|v| v.name.clone())
            .chain(module.equations.iter().map(|v| v.name.clone()))
            .chain(module.proofs.iter().map(|v| v.name.clone()))
            .chain(module.clauses.iter().map(|v| v.name.clone()))
            .chain(module.inductions.iter().map(|v| v.name.clone()))
            .collect();

        while self.current().kind != Kind::End {
            let start = self.cursor;
            let keyword = self.identifier()?;
            let kind = DeclarationKind::from_keyword(&keyword).filter(|kind| kind.is_top_level())
                .ok_or_else(|| self.fail(format!("unknown declaration '{keyword}'; expected rule, axiom, proof, infer, query, or eval"))
                    .with_kind(ParseErrorKind::UnknownDeclaration(keyword)))?;

            match kind {
                DeclarationKind::Record => self.record_type_declaration(&mut module)?,
                DeclarationKind::Monad => self.monad_declaration(&mut module)?,
                DeclarationKind::Import => self.import_declaration(&mut module)?,
                DeclarationKind::Domain => self.domain_declaration(&mut module)?,
                DeclarationKind::Map => self.map_declaration(&mut module)?,
                DeclarationKind::Fixed => {
                    module.fixed_points.push(self.identifier()?);
                    self.symbol(';')?;
                }
                DeclarationKind::Induction => {
                    self.induction_declaration(&mut module, &mut names)?
                }
                DeclarationKind::Sort | DeclarationKind::Datatype => {
                    self.sort_declaration(&mut module, kind == DeclarationKind::Datatype)?;
                }
                DeclarationKind::Function => self.function_declaration(&mut module, &mut names)?,
                DeclarationKind::Constructor => {
                    let name = self.identifier()?;
                    let arguments = self.sorts()?;
                    self.symbol(':')?;
                    let result = self.sort()?;
                    self.symbol(';')?;
                    let signature = Signature {
                        name,
                        arguments,
                        result,
                        kind: SignatureKind::Constructor,
                    };
                    self.signatures.push(signature.clone());
                    module.signatures.push(signature);
                }
                DeclarationKind::Judgment => self.judgment_declaration(&mut module)?,
                DeclarationKind::Notation => self.notation_declaration(&mut module)?,
                DeclarationKind::Eval | DeclarationKind::Query => {
                    let term = self.term(0)?;
                    self.symbol(';')?;

                    if kind == DeclarationKind::Eval {
                        module.evaluations.push(term);
                    } else {
                        module.queries.push(term);
                    }
                }
                DeclarationKind::Rule => self.definition(
                    &mut module,
                    &mut names,
                    crate::definitions::DefinitionKind::Rule,
                )?,
                DeclarationKind::Axiom => self.definition(
                    &mut module,
                    &mut names,
                    crate::definitions::DefinitionKind::Axiom,
                )?,
                DeclarationKind::Proof => self.definition(
                    &mut module,
                    &mut names,
                    crate::definitions::DefinitionKind::Proof,
                )?,
                DeclarationKind::Infer => self.definition(
                    &mut module,
                    &mut names,
                    crate::definitions::DefinitionKind::Infer,
                )?,
                DeclarationKind::Module | DeclarationKind::Case => {
                    unreachable!("filtered non-top-level declarations")
                }
            }

            self.record_declaration(start);
        }

        Ok(module)
    }
}
