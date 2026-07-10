use soda_ast::{Module, Term};
use std::collections::BTreeSet;
mod declarations;
mod definitions;
mod diagnostic;
mod error;
mod formatter;
mod functions;
mod imports;
pub mod lexer;
pub use formatter::format_source;
mod locations;
mod module;
mod notation;
mod proofs;
mod render;
mod sorts;
mod surface;
mod symbols;
mod terms;
use error::error;
pub use error::{ParseError, ParseErrorKind};
pub use imports::{LoadError, LoadedModule, LoadedSource, load_module, load_module_source};
use lexer::{Kind, Token, lex};
pub use locations::{
    DeclarationKind, DeclarationLocation, Locations, ParsedModule, ParsedTerm, TermLocation,
};
use notation::{Notation, Part};
pub use render::{NotationStyle, format_term_in};
const MAX_TERM_DEPTH: usize = 256;
struct Parser<'a> {
    source: &'a str,
    imported: Module,
    tokens: Vec<Token>,
    cursor: usize,
    notations: Vec<Notation>,
    locations: Locations,
    track_locations: bool,
    records: Vec<soda_ast::RecordDecl>,
    monads: Vec<soda_ast::MonadDecl>,
    signatures: Vec<soda_ast::Signature>,
    do_names: Vec<String>,
}
impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Result<Self, Box<ParseError>> {
        Ok(Self {
            source,
            imported: Module::default(),
            tokens: lex(source)?,
            cursor: 0,
            notations: Vec::new(),
            locations: Locations::default(),
            track_locations: false,
            records: Vec::new(),
            monads: Vec::new(),
            signatures: Vec::new(),
            do_names: Vec::new(),
        })
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn fail(&self, message: impl Into<String>) -> Box<ParseError> {
        error(self.source, self.current().offset, message)
    }

    fn expected(&self, message: impl Into<String>) -> Box<ParseError> {
        self.fail(message).with_kind(ParseErrorKind::ExpectedSyntax)
    }

    fn eat(&mut self, kind: &Kind) -> bool {
        if &self.current().kind == kind {
            self.cursor += 1;

            true
        } else {
            false
        }
    }

    fn symbol(&mut self, ch: char) -> Result<(), Box<ParseError>> {
        if self.eat(&Kind::Symbol(ch)) {
            Ok(())
        } else {
            Err(self
                .fail(format!("expected '{ch}'"))
                .with_kind(ParseErrorKind::ExpectedToken(ch.to_string())))
        }
    }

    fn keyword(&mut self, word: &str) -> bool {
        self.eat(&Kind::Ident(word.into()))
    }

    fn identifier(&mut self) -> Result<String, Box<ParseError>> {
        if let Kind::Ident(name) = self.current().kind.clone() {
            self.cursor += 1;

            Ok(name)
        } else {
            Err(self.expected("expected an identifier"))
        }
    }

    fn text(&mut self) -> Result<String, Box<ParseError>> {
        if let Kind::Text(text) = self.current().kind.clone() {
            self.cursor += 1;

            Ok(text)
        } else {
            Err(self.expected("expected a quoted notation template"))
        }
    }

    fn unique_name(&mut self, names: &mut BTreeSet<String>) -> Result<String, Box<ParseError>> {
        let offset = self.current().offset;
        let name = self.identifier()?;

        if !names.insert(name.clone()) {
            return Err(error(
                self.source,
                offset,
                format!("duplicate declaration '{name}'"),
            )
            .with_kind(ParseErrorKind::DuplicateDeclaration(name)));
        }

        Ok(name)
    }

    fn previous_end(&self) -> usize {
        self.tokens[self.cursor.saturating_sub(1)].end
    }
}

pub fn parse_module(source: &str) -> Result<Module, Box<ParseError>> {
    Parser::new(source)?.module()
}

/// Parse with exact byte ranges for declarations, subterms, and certificate steps.
pub fn parse_module_located(source: &str) -> Result<ParsedModule, Box<ParseError>> {
    let mut parser = Parser::new(source)?;
    parser.track_locations = true;
    let module = parser.module()?;

    Ok(ParsedModule {
        module,
        locations: parser.locations,
    })
}

pub fn parse_term(source: &str) -> Result<Term, Box<ParseError>> {
    parse_term_in(&Module::default(), source)
}

pub fn parse_term_in(module: &Module, source: &str) -> Result<Term, Box<ParseError>> {
    parse_term_with_locations(module, source, false).map(|parsed| parsed.term)
}

pub fn parse_term_located_in(module: &Module, source: &str) -> Result<ParsedTerm, Box<ParseError>> {
    parse_term_with_locations(module, source, true)
}

fn parse_term_with_locations(
    module: &Module,
    source: &str,
    track_locations: bool,
) -> Result<ParsedTerm, Box<ParseError>> {
    let mut parser = Parser::new(source)?;
    parser.track_locations = track_locations;
    parser.records = module.records.clone();
    parser.monads = module.monads.clone();
    parser.signatures = module.signatures.clone();

    for notation in &module.notations {
        parser.add_notation(module, notation)?;
    }

    let term = parser.term(0)?;

    if parser.current().kind != Kind::End {
        return Err(parser.fail("unexpected text after term"));
    }

    Ok(ParsedTerm {
        term,
        locations: parser.locations,
    })
}
