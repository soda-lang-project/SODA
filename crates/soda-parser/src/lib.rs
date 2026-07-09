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
pub use imports::{LoadError, LoadedModule, LoadedSource};
use lexer::{Kind, Token, lex};
pub use locations::{DeclarationKind, DeclarationLocation, Locations, ParsedModule, ParsedTerm, TermLocation};
use notation::{Notation, Part};
pub use render::{NotationStyle};
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

    fn text(&mut self) -> Result<String, Box<ParseError>> {
        if let Kind::Text(text) = self.current().kind.clone() {
            self.cursor += 1;

            Ok(text)
        } else {
            Err(self.expected("expected a quoted notation template"))
        }
    }

}

