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

