pub(super) enum Doc<'a> {
    Text(&'a str),
    Concat(Vec<Self>),
    Line(&'static str),
    HardLine,
    BlankLine,
    Nest(Box<Self>),
    Align(Box<Self>),
    Group { width: usize, body: Box<Self> },
}
impl<'a> Doc<'a> {

    pub(super) fn width(&self) -> usize {
        match self {
            Self::Text(text) if !text.contains(['\r', '\n']) => text.chars().count(),
            Self::Line(text) => text.len(),
            Self::Concat(parts) => parts
                .iter()
                .fold(0usize, |width, part| width.saturating_add(part.width())),
            Self::Group { width, .. } => *width,
            Self::Nest(body) | Self::Align(body) => body.width(),
            _ => usize::MAX,
        }
    }
    fn prefix(&self, flat: bool) -> (usize, bool) {
        match self {
            Self::Line(text) if flat => (text.len(), false),
            Self::Line(_) | Self::HardLine | Self::BlankLine => (0, true),
            Self::Nest(body) | Self::Align(body) => body.prefix(flat),
            Self::Concat(parts) => {
                let mut width = 0usize;
                for part in parts {
                    let (next, breaks) = part.prefix(flat);
                    width = width.saturating_add(next);
                    if breaks {
                        return (width, true);
                    }
                }
                (width, false)
            }
            Self::Group { width, .. } if *width != usize::MAX => (*width, false),
            Self::Group { body, .. } => body.prefix(false),
            Self::Text(text) => (
                text.lines().next().unwrap_or("").chars().count(),
                text.contains('\n'),
            ),
        }
    }
}

struct Writer<'a> {
    text: String,
    column: usize,
    newline: &'a str,
}
impl Writer<'_> {

}
