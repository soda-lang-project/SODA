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

fn standard_library(name: &str) -> Option<&'static str> {
    match name {
        "soda.languages.arithmetic" => Some(include_str!("../stdlib/arithmetic.soda")),
        "soda.prelude" => Some(include_str!("../stdlib/prelude.soda")),
        "soda.data.nat" => Some(include_str!("../stdlib/nat.soda")),
        "soda.math.rational" => Some(include_str!("../stdlib/rational.soda")),
        "soda.probability" => Some(include_str!("../stdlib/probability.soda")),
        "soda.effects.state" => Some(include_str!("../stdlib/state.soda")),
        "soda.effects.exceptions" => Some(include_str!("../stdlib/exceptions.soda")),
        "soda.collections.sets" => Some(include_str!("../stdlib/sets.soda")),
        "soda.collections" => Some(include_str!("../stdlib/collections.soda")),
        _ => None,
    }
}

fn merge(target: &mut Module, source: &Module) {
    macro_rules! fields {
        ($($field:ident),*) => { $(target.$field.extend(source.$field.iter().cloned());)* };
    }
    fields!(
        rules,
        equations,
        proofs,
        clauses,
        sorts,
        signatures,
        judgments,
        notations,
        records,
        monads,
        domains,
        semantic_maps,
        fixed_points,
        inductions,
        clause_details
    );
    // Imported examples are not executed as inputs to the importing program.
}

fn duplicate_export(target: &Module, source: &Module) -> Option<String> {
    fn definitions(module: &Module) -> Vec<&str> {
        module
            .rules
            .iter()
            .map(|v| v.name.as_str())
            .chain(module.equations.iter().map(|v| v.name.as_str()))
            .chain(module.proofs.iter().map(|v| v.name.as_str()))
            .chain(module.clauses.iter().map(|v| v.name.as_str()))
            .chain(module.inductions.iter().map(|v| v.name.as_str()))
            .collect()
    }
    let namespaces = [
        (definitions(target), definitions(source)),
        (
            target.sorts.iter().map(|v| v.name.as_str()).collect(),
            source.sorts.iter().map(|v| v.name.as_str()).collect(),
        ),
        (
            target
                .signatures
                .iter()
                .map(|v| v.name.as_str())
                .chain(target.judgments.iter().map(|v| v.name.as_str()))
                .collect(),
            source
                .signatures
                .iter()
                .map(|v| v.name.as_str())
                .chain(source.judgments.iter().map(|v| v.name.as_str()))
                .collect(),
        ),
        (
            target.domains.iter().map(|v| v.name.as_str()).collect(),
            source.domains.iter().map(|v| v.name.as_str()).collect(),
        ),
        (
            target
                .semantic_maps
                .iter()
                .map(|v| v.name.as_str())
                .collect(),
            source
                .semantic_maps
                .iter()
                .map(|v| v.name.as_str())
                .collect(),
        ),
        (
            target.monads.iter().map(|v| v.name.as_str()).collect(),
            source.monads.iter().map(|v| v.name.as_str()).collect(),
        ),
    ];
    for (previous, incoming) in namespaces {
        let names: BTreeSet<_> = previous.into_iter().collect();
        if let Some(name) = incoming.into_iter().find(|name| names.contains(name)) {
            return Some(name.into());
        }
    }
    None
}

fn local_part(mut module: Module, prelude: &Module) -> Module {
    macro_rules! fields {
        ($($field:ident),*) => { $(module.$field.drain(..prelude.$field.len());)* };
    }
    fields!(
        rules,
        equations,
        proofs,
        clauses,
        sorts,
        signatures,
        judgments,
        notations,
        records,
        monads,
        domains,
        semantic_maps,
        fixed_points,
        inductions,
        clause_details
    );
    module.imports.clear();
    module
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
    fn visit(
        &mut self,
        key: String,
        name: String,
        text: String,
        base: &Path,
    ) -> Result<Vec<String>, LoadError> {
        let failure = |message| LoadError {
            source: name.clone(),
            text: Some(text.clone()),
            span: None,
            parse: None,
            message,
        };
        if self.active.contains(&key) {
            let mut cycle = self.active.clone();
            cycle.push(key);
            return Err(failure(format!("cyclic imports: {}", cycle.join(" -> "))));
        }
        if let Some(node) = self.nodes.get(&key) {
            return Ok(node.order.clone());
        }
        if self.nodes.len() + self.active.len() >= MAX_MODULES
            || self.bytes + text.len() > MAX_SOURCE_BYTES
        {
            return Err(failure(
                "import resource limit exceeded (128 modules or 8 MiB of source)".into(),
            ));
        }
        self.bytes += text.len();
        self.active.push(key.clone());
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();
        for (imported, span) in imports(&name, &text)? {
            let dependency = if let Some(source) = standard_library(&imported) {
                self.visit(
                    format!("stdlib:{imported}"),
                    format!("<{imported}>"),
                    source.into(),
                    base,
                )
            } else {
                let relative = if imported.ends_with(".soda")
                    || imported.contains('/')
                    || imported.contains('\\')
                {
                    PathBuf::from(&imported)
                } else {
                    PathBuf::from(imported.replace('.', "/")).with_extension("soda")
                };
                let path = base.join(relative);
                let resolved = path.canonicalize().map_err(|e| {
                    let mut error = failure(format!(
                        "cannot import `{imported}` from {}: {e}",
                        path.display()
                    ));
                    error.span = Some(span);
                    error
                })?;
                let contents = std::fs::read_to_string(&resolved).map_err(|e| {
                    let mut error = failure(format!("cannot read import `{imported}`: {e}"));
                    error.span = Some(span);
                    error
                })?;
                self.visit(
                    resolved.to_string_lossy().into_owned(),
                    path.to_string_lossy().into_owned(),
                    contents,
                    resolved.parent().unwrap_or(base),
                )
            }?;
            for dependency in dependency {
                if seen.insert(dependency.clone()) {
                    order.push(dependency);
                }
            }
        }
        let mut prelude = Module::default();
        for dependency in &order {
            let node = &self.nodes[dependency];
            if let Some(duplicate) = duplicate_export(&prelude, &node.local) {
                let source = &self.sources[node.source];
                return Err(LoadError {
                    source: source.name.clone(),
                    text: Some(source.text.clone()),
                    span: source
                        .locations
                        .declaration(&duplicate)
                        .map(|site| site.name_span),
                    parse: None,
                    message: format!(
                        "duplicate imported declaration `{duplicate}` from distinct modules"
                    ),
                });
            }
            merge(&mut prelude, &node.local);
        }
        let mut parser = Parser::new(&text).map_err(|e| parse_failure(&name, &text, e))?;
        parser.track_locations = true;
        parser.records = prelude.records.clone();
        parser.monads = prelude.monads.clone();
        parser.signatures = prelude.signatures.clone();
        for notation in &prelude.notations {
            parser
                .add_notation(&prelude, notation)
                .map_err(|e| parse_failure(&name, &text, e))?;
        }
        parser.imported = prelude.clone();
        let parsed = parser
            .module()
            .map_err(|e| parse_failure(&name, &text, e))?;
        let locations = parser.locations;
        let local = local_part(parsed, &prelude);
        order.push(key.clone());
        self.nodes.insert(
            key,
            Node {
                local,
                order: order.clone(),
                source: self.sources.len(),
            },
        );
        self.sources.push(LoadedSource {
            name,
            text,
            locations,
        });
        self.active.pop();
        Ok(order)
    }
}

/// Parse and resolve a source string. Relative imports use `base`.
pub fn load_module_source(
    name: &str,
    text: &str,
    base: impl AsRef<Path>,
) -> Result<LoadedModule, LoadError> {
    let mut loader = Loader::default();
    let key = Path::new(name)
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| format!("source:{name}"));
    let order = loader.visit(key, name.into(), text.into(), base.as_ref())?;
    let root = loader.sources.len() - 1;
    let root_module = &loader.nodes[order.last().expect("root exists")].local;
    let mut module = Module {
        name: root_module.name.clone(),
        evaluations: root_module.evaluations.clone(),
        queries: root_module.queries.clone(),
        ..Module::default()
    };
    for key in order {
        merge(&mut module, &loader.nodes[&key].local);
    }
    Ok(LoadedModule {
        module,
        sources: loader.sources,
        root,
    })
}

pub fn load_module(path: impl AsRef<Path>) -> Result<LoadedModule, LoadError> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|error| LoadError {
        source: path.display().to_string(),
        text: None,
        span: None,
        parse: None,
        message: error.to_string(),
    })?;
    load_module_source(
        &path.to_string_lossy(),
        &text,
        path.parent().unwrap_or(Path::new(".")),
    )
}
