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
    pub fn concat(parts: Vec<Self>) -> Self {
        Self::Concat(parts)
    }
    pub fn group(self) -> Self {
        Self::Group {
            width: self.width(),
            body: Box::new(self),
        }
    }
    pub fn nest(self) -> Self {
        Self::Nest(Box::new(self))
    }
    pub fn align(self) -> Self {
        Self::Align(Box::new(self))
    }
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

pub(super) fn render(doc: &Doc<'_>, newline: &str) -> String {
    let mut writer = Writer {
        text: String::new(),
        column: 0,
        newline,
    };
    writer.doc(doc, 0, false, 0);
    while writer.text.ends_with(newline) {
        writer.text.truncate(writer.text.len() - newline.len());
    }
    if !writer.text.is_empty() {
        writer.text.push_str(newline);
    }
    writer.text
}
struct Writer<'a> {
    text: String,
    column: usize,
    newline: &'a str,
}
impl Writer<'_> {
    fn line(&mut self, blank: bool) {
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        if !self.text.is_empty() && !self.text.ends_with(self.newline) {
            self.text.push_str(self.newline);
        }
        if blank && !self.text.is_empty() && !self.text.ends_with(&self.newline.repeat(2)) {
            self.text.push_str(self.newline);
        }
        self.column = 0;
    }
    fn doc(&mut self, doc: &Doc<'_>, indent: usize, flat: bool, following: usize) {
        match doc {
            Doc::Text(text) => {
                if text.is_empty() {
                    return;
                }
                if self.column == 0 {
                    self.text.push_str(&" ".repeat(indent));
                    self.column = indent;
                }
                self.text.push_str(text);
                self.column = if let Some((_, tail)) = text.rsplit_once('\n') {
                    tail.chars().count()
                } else {
                    self.column + text.chars().count()
                };
            }
            Doc::Concat(parts) => {
                let mut remaining = following;
                let mut suffixes = Vec::with_capacity(parts.len());
                for part in parts.iter().rev() {
                    suffixes.push(remaining);
                    let (width, breaks) = part.prefix(flat);
                    remaining = if breaks {
                        width
                    } else {
                        remaining.saturating_add(width)
                    };
                }
                for (part, suffix) in parts.iter().zip(suffixes.into_iter().rev()) {
                    self.doc(part, indent, flat, suffix);
                }
            }
            Doc::Line(text) if flat => self.doc(&Doc::Text(text), indent, true, 0),
            Doc::Line(_) | Doc::HardLine => self.line(false),
            Doc::BlankLine => self.line(true),
            Doc::Nest(body) => self.doc(body, indent + 4, flat, following),
            Doc::Align(body) => self.doc(body, self.column.max(indent), flat, following),
            Doc::Group { width, body } => self.doc(
                body,
                indent,
                flat || width.saturating_add(following)
                    <= 100usize.saturating_sub(self.column.max(indent)),
                following,
            ),
        }
    }
}
