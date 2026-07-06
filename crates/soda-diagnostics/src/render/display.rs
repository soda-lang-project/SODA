use super::TAB_WIDTH;
use crate::Span;
use unicode_width::UnicodeWidthChar;

pub fn safe(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if unsafe_control(ch) {
                format!("\\u{{{:x}}}", ch as u32)
            } else {
                ch.to_string()
            }
        })
        .collect()
}

fn unsafe_control(ch: char) -> bool {
    ch.is_control()
        || matches!(ch as u32, 0x061c | 0x200b..=0x200f | 0x2028..=0x202e | 0x2060..=0x206f | 0xfeff)
}

/// Maps source byte offsets to escaped display offsets.
pub(super) struct DisplaySource {
    pub(super) text: String,
    offsets: Vec<(usize, usize)>,
}

impl DisplaySource {

    pub(super) fn new(source: &str) -> Self {
        let mut text = String::new();
        let mut offsets = Vec::new();
        let mut column = 0;

        for (byte, ch) in source.char_indices() {
            offsets.push((byte, text.len()));

            match ch {
                '\n' => {
                    text.push(ch);
                    column = 0;
                }
                '\r' if source[byte..].starts_with("\r\n") => text.push(ch),
                '\t' => {
                    let count = TAB_WIDTH - column % TAB_WIDTH;
                    text.push_str(&" ".repeat(count));
                    column += count;
                }
                ch if unsafe_control(ch) => {
                    let escaped = safe(&ch.to_string());
                    column += escaped.len();
                    text.push_str(&escaped);
                }
                ch => {
                    text.push(ch);
                    column += ch.width().unwrap_or(0);
                }
            }
        }

        offsets.push((source.len(), text.len()));

        Self { text, offsets }
    }

    pub(super) fn offset(&self, original: usize) -> usize {
        self.offsets[self
            .offsets
            .partition_point(|&(byte, _)| byte <= original)
            .saturating_sub(1)]
        .1
    }

}
