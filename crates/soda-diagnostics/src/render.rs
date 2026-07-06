use crate::{Diagnostic, Label, Severity, SourceFile, SourceMap, Span};
use annotate_snippets::renderer::DecorStyle;
use annotate_snippets::{AnnotationKind, Group, Level, Patch, Renderer, Snippet};
mod display;
use display::DisplaySource;
pub use display::safe;

pub const DEFAULT_RENDER_WIDTH: usize = 100;
pub const MIN_RENDER_WIDTH: usize = 40;
pub const MAX_RENDER_WIDTH: usize = 240;
/// Space reserved for gutters and annotation labels when cropping source lines.
const ANNOTATION_MARGIN: usize = 20;
const TAB_WIDTH: usize = 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Style {
    Unicode,
    #[default]
    Ascii,
}

#[derive(Clone, Debug)]
pub struct RenderOptions {
    pub color: bool,
    pub style: Style,
    pub width: usize,
    pub context_lines: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            color: false,
            style: Style::Ascii,
            width: DEFAULT_RENDER_WIDTH,
            context_lines: 1,
        }
    }
}

fn snippet(
    source: &SourceFile,
    display: &DisplaySource,
) -> Snippet<'static, annotate_snippets::Annotation<'static>> {
    Snippet::source(display.text.clone())
        .path(safe(&source.name))
        .fold(true)
}

/// Split distant annotations so the backend can crop each snippet.
fn label_windows<'a>(
    source: &SourceFile,
    labels: Vec<&'a Label>,
    width: usize,
) -> Vec<Vec<&'a Label>> {
    let budget = width
        .clamp(MIN_RENDER_WIDTH, MAX_RENDER_WIDTH)
        .saturating_sub(ANNOTATION_MARGIN);
    let mut windows: Vec<Vec<&Label>> = Vec::new();

    for label in labels {
        let position = source.position(label.span.start);
        let window = windows.iter_mut().find(|window| {
            window.iter().all(|other| {
                let other_position = source.position(other.span.start);

                position.0 != other_position.0 || position.1.abs_diff(other_position.1) <= budget
            })
        });

        if let Some(window) = window {
            window.push(label);
        } else {
            windows.push(vec![label]);
        }
    }

    windows
}

pub fn render(diagnostic: &Diagnostic, sources: &SourceMap, options: &RenderOptions) -> String {
    let level = match diagnostic.severity {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
        Severity::Note => Level::NOTE,
    };

    let mut primary = Group::with_title(
        level
            .primary_title(safe(&diagnostic.title))
            .id(safe(&diagnostic.code)),
    );
    let mut source_ids = Vec::new();

    for label in diagnostic
        .labels
        .iter()
        .filter(|label| label.primary)
        .chain(diagnostic.labels.iter())
    {
        if !source_ids.contains(&label.source) {
            source_ids.push(label.source);
        }
    }

    for id in source_ids {
        let Some(source) = sources.get(id) else {
            continue;
        };

        let labels = diagnostic
            .labels
            .iter()
            .filter(|label| label.source == id)
            .collect::<Vec<_>>();
        let display = DisplaySource::file(
            &source.text,
            labels
                .iter()
                .any(|label| source.clamp(label.span).start == source.text.len()),
        );

        for labels in label_windows(source, labels, options.width) {
            let mut view = snippet(source, &display);

            for label in labels {
                let span = source.clamp(label.span);
                let kind = if label.primary {
                    AnnotationKind::Primary
                } else {
                    AnnotationKind::Context
                };

                view = view.annotation(kind.span(display.range(span)).label(safe(&label.message)));
                let context = options.context_lines.min(3);

                for line in [source.position(span.start).0, source.position(span.end).0] {
                    for number in line.saturating_sub(context).max(1)
                        ..=(line + context).min(source.line_count())
                    {
                        let (start, text) = source.line(number);
                        view = view.annotation(
                            AnnotationKind::Visible
                                .span(display.range(Span::new(start, start + text.len()))),
                        );
                    }
                }
            }
            primary = primary.element(view);
        }
    }

    for note in &diagnostic.notes {
        primary = primary.element(Level::NOTE.message(safe(note)));
    }

    for help in &diagnostic.help {
        primary = primary.element(Level::HELP.message(safe(help)));
    }

    let mut groups = vec![primary];

    for suggestion in &diagnostic.suggestions {
        let title = Level::HELP.secondary_title(safe(&suggestion.message));

        if let Some(source) = sources.get(suggestion.source) {
            let display = DisplaySource::file(
                &source.text,
                source.clamp(suggestion.span).start == source.text.len(),
            );
            groups.push(
                title.element(
                    Snippet::source(display.text.clone())
                        .path(safe(&source.name))
                        .patch(Patch::new(
                            display.range(source.clamp(suggestion.span)),
                            DisplaySource::new(&suggestion.replacement).text,
                        )),
                ),
            );
        } else {
            groups.push(title.element(Level::NOTE.message(safe(&suggestion.replacement))));
        }
    }

    let light = |color: anstyle::AnsiColor| anstyle::Style::new().fg_color(Some(color.into()));
    let renderer = if options.color {
        Renderer::styled()
            .error(light(anstyle::AnsiColor::BrightRed).bold())
            .warning(light(anstyle::AnsiColor::BrightYellow).bold())
            .info(light(anstyle::AnsiColor::BrightCyan).bold())
            .note(light(anstyle::AnsiColor::BrightCyan))
            .help(light(anstyle::AnsiColor::BrightGreen))
            .line_num(light(anstyle::AnsiColor::BrightCyan))
            .context(light(anstyle::AnsiColor::BrightCyan))
    } else {
        Renderer::plain()
    }
    .term_width(options.width.clamp(MIN_RENDER_WIDTH, MAX_RENDER_WIDTH))
    .decor_style(match options.style {
        Style::Unicode => DecorStyle::Unicode,
        Style::Ascii => DecorStyle::Ascii,
    });

    format!("{}\n", renderer.render(&groups))
}
