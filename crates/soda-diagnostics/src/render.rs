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

