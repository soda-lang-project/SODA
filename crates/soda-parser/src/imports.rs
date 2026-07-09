use crate::{Kind, Locations, ParseError, Parser};
use soda_ast::Module;
use soda_diagnostics::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MAX_MODULES: usize = 128;
const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct LoadedSource {
    pub name: String,
    pub text: String,
    pub locations: Locations,
}

#[derive(Clone, Debug)]
pub struct LoadedModule {
    pub module: Module,
    /// Each source occurs once, dependencies before their importing source.
    pub sources: Vec<LoadedSource>,
    pub root: usize,
}

#[derive(Clone, Debug)]
pub struct LoadError {
    pub source: String,
    pub text: Option<String>,
    pub span: Option<Span>,
    pub parse: Option<Box<ParseError>>,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.source, self.message)
    }
}
impl std::error::Error for LoadError {}

impl Parser<'_> {
    fn import_name(&mut self) -> Result<String, Box<ParseError>> {
        if matches!(self.current().kind, Kind::Text(_)) {
            return self.text();
        }
        let mut name = self.identifier()?;
        while self.eat(&Kind::Symbol('.')) {
            name.push('.');
            name.push_str(&self.identifier()?);
        }
        Ok(name)
    }

    pub(super) fn import_declaration(
        &mut self,
        module: &mut Module,
    ) -> Result<(), Box<ParseError>> {
        let start = self.tokens[self.cursor - 1].offset;
        let declared = imports("<source>", self.source)
            .map_err(|error| error.parse.unwrap_or_else(|| self.fail(error.message)))?;
        if !declared.iter().any(|(_, span)| span.start == start) {
            return Err(self.expected("imports must appear immediately after the module header, before local declarations"));
        }
        module.imports.push(self.import_name()?);
        self.symbol(';')
    }
}

fn parse_failure(name: &str, text: &str, error: Box<ParseError>) -> LoadError {
    LoadError {
        source: name.into(),
        text: Some(text.into()),
        span: None,
        message: error.to_string(),
        parse: Some(error),
    }
}

fn imports(name: &str, text: &str) -> Result<Vec<(String, Span)>, LoadError> {
    let mut parser = Parser::new(text).map_err(|e| parse_failure(name, text, e))?;
    if !parser.keyword("module") {
        return Err(parse_failure(
            name,
            text,
            parser.expected("expected 'module' declaration"),
        ));
    }
    parser
        .identifier()
        .map_err(|e| parse_failure(name, text, e))?;
    parser
        .symbol(';')
        .map_err(|e| parse_failure(name, text, e))?;
    let mut result = Vec::new();
    while parser.current().kind == Kind::Ident("import".into()) {
        let start = parser.current().offset;
        parser.cursor += 1;
        let imported = parser
            .import_name()
            .map_err(|e| parse_failure(name, text, e))?;
        parser
            .symbol(';')
            .map_err(|e| parse_failure(name, text, e))?;
        result.push((imported, Span::new(start, parser.previous_end())));
    }
    Ok(result)
}

struct Node {
    local: Module,
    order: Vec<String>,
    source: usize,
}
#[derive(Default)]
struct Loader {
    nodes: BTreeMap<String, Node>,
    active: Vec<String>,
    sources: Vec<LoadedSource>,
    bytes: usize,
}

impl Loader {

}

