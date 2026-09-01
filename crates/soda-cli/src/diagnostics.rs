use soda_ast::{Module, Term};
use soda_diagnostics::{Diagnostic, Report, SourceId, Span};
use soda_parser::{DeclarationLocation, Locations, load_module_source, parse_term_located_in};
use std::fs;
use std::io::{self, Read};

mod options;
mod runtime;
mod sorts;
mod verification;
pub(crate) use options::{ColorMode, DiagnosticFormat, Options, argument_error};

pub struct Context {
    pub module: Module,
    pub report: Report,
    pub locations: Locations,
    pub source: SourceId,
    argument: Option<(SourceId, Locations)>,
    imported_sources: Vec<(SourceId, Locations)>,
    handled_error: bool,
}

impl Context {
    pub fn read(file: &str) -> Result<Self, Report> {
        let result = if file == "-" {
            let mut source = String::new();

            io::stdin().read_to_string(&mut source).map(|_| source)
        } else {
            fs::read_to_string(file)
        };

        let mut report = Report::default();
        let text = result.map_err(|error| {
            report.diagnostics.push(
                Diagnostic::error("E0101", format!("could not read `{file}`"))
                    .note(error.to_string())
                    .help("check the source path and that the file contains UTF-8 text"),
            );

            report.clone()
        })?;
        Self::from_source(file, &text)
    }

    pub fn from_source(file: &str, text: &str) -> Result<Self, Report> {
        let mut report = Report::default();
        let source = report
            .sources
            .add(if file == "-" { "<stdin>" } else { file }, text);
        let base = if file == "-" {
            std::path::Path::new(".")
        } else {
            std::path::Path::new(file)
                .parent()
                .unwrap_or(std::path::Path::new("."))
        };
        let loaded = load_module_source(if file == "-" { "<stdin>" } else { file }, text, base)
            .map_err(|error| {
                let id = if error.source == file || file == "-" && error.source == "<stdin>" {
                    source
                } else {
                    report
                        .sources
                        .add(&error.source, error.text.as_deref().unwrap_or_default())
                };
                let diagnostic = if let Some(parse) = &error.parse {
                    parse.diagnostic(id, error.text.as_deref().unwrap_or_default())
                } else {
                    let mut diagnostic = Diagnostic::error("E0102", &error.message);
                    if let Some(span) = error.span {
                        diagnostic = diagnostic.primary(id, span, &error.message);
                    }
                    diagnostic
                };
                report.diagnostics.push(diagnostic);
                report.clone()
            })?;
        let mut locations = Locations::default();
        let mut imported_sources = Vec::new();
        for (index, loaded_source) in loaded.sources.into_iter().enumerate() {
            if index == loaded.root {
                locations = loaded_source.locations;
            } else {
                let id = report.sources.add(loaded_source.name, loaded_source.text);
                imported_sources.push((id, loaded_source.locations));
            }
        }

        Ok(Self {
            module: loaded.module,
            report,
            locations,
            source,
            argument: None,
            imported_sources,
            handled_error: false,
        })
    }

    pub fn parse_input(&mut self, text: &str) -> Result<Term, String> {
        let source = self.report.sources.add("<term argument>", text);

        match parse_term_located_in(&self.module, text) {
            Ok(parsed) => {
                self.argument = Some((source, parsed.locations));

                Ok(parsed.term)
            }
            Err(error) => {
                let diagnostic = error.diagnostic(source, text);

                Err(self.error(diagnostic, error.to_string()))
            }
        }
    }

    pub fn error(&mut self, diagnostic: Diagnostic, message: String) -> String {
        self.report.diagnostics.push(diagnostic);
        self.handled_error = true;

        message
    }

    pub fn ground_error(&mut self, term: &Term) -> String {
        let (source, span) = self.locate(Some(term), None);

        self.error(Diagnostic::error("E1201", "evaluation requires a ground term")
            .primary(source, span, "this input still contains rule metavariables")
            .help("replace each `?variable` with a concrete term, or use an inference query to search for bindings"), "evaluation requires a ground term".into())
    }

    fn locate(&self, term: Option<&Term>, declaration: Option<&str>) -> (SourceId, Span) {
        if let Some((source, locations)) = &self.argument
            && let Some(span) = term.and_then(|term| locations.term(term, None))
        {
            return (*source, span);
        }

        if let Some((source, site)) = declaration.and_then(|name| self.declaration_site(name)) {
            let span = term
                .and_then(|term| self.source_locations(source).term(term, Some(site.span)))
                .unwrap_or(site.name_span);
            return (source, span);
        }

        let site = declaration.and_then(|name| self.locations.declaration(name));

        if let Some(term) = term {
            for site in self.locations.declarations.iter().filter(|site| {
                site.name.as_deref() == declaration || declaration.is_none() && site.kind.is_input()
            }) {
                if let Some(span) = self.locations.term(term, Some(site.span)) {
                    return (self.source, span);
                }
            }

            if let Some(span) = self.locations.term(term, None) {
                return (self.source, span);
            }
        }

        if let Some(site) = site {
            return (self.source, site.name_span);
        }

        (self.source, Span::new(0, 0))
    }

    fn declaration_site(&self, name: &str) -> Option<(SourceId, &DeclarationLocation)> {
        self.locations
            .declaration(name)
            .map(|site| (self.source, site))
            .or_else(|| {
                self.imported_sources
                    .iter()
                    .find_map(|(source, locations)| {
                        locations.declaration(name).map(|site| (*source, site))
                    })
            })
    }

    pub(crate) fn source_locations(&self, source: SourceId) -> &Locations {
        if let Some((id, locations)) = &self.argument
            && *id == source
        {
            return locations;
        }
        self.imported_sources
            .iter()
            .find(|(id, _)| *id == source)
            .map(|(_, locations)| locations)
            .unwrap_or(&self.locations)
    }
}
