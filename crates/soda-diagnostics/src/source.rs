#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

pub type SourceId = usize;

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    lines: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut lines = vec![0];
        lines.extend(text.match_indices('\n').map(|(index, _)| index + 1));

        Self {
            name: name.into(),
            text,
            lines,
        }
    }

    pub fn clamp(&self, span: Span) -> Span {
        let mut start = span.start.min(self.text.len());

        while !self.text.is_char_boundary(start) {
            start -= 1;
        }

        let mut end = span.end.max(start).min(self.text.len());

        while !self.text.is_char_boundary(end) {
            end += 1;
        }

        Span::new(start, end)
    }

    /// One-based line and Unicode scalar column; ranges remain UTF-8 byte offsets.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let offset = self.clamp(Span::new(offset, offset)).start;
        let line = self
            .lines
            .partition_point(|&start| start <= offset)
            .saturating_sub(1);

        (
            line + 1,
            self.text[self.lines[line]..offset].chars().count() + 1,
        )
    }

    pub(crate) fn line(&self, number: usize) -> (usize, &str) {
        let start = self.lines[number - 1];
        let end = self.lines.get(number).copied().unwrap_or(self.text.len());

        (start, self.text[start..end].trim_end_matches(['\r', '\n']))
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let id = self.files.len();
        self.files.push(SourceFile::new(name, text));

        id
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id)
    }
}
