mod conditional;
mod document;
mod layout;
use crate::{ParseError, lexer::{Kind, Token, lex}};
use document::Doc;

/// Formats at 100 columns with four-space indentation, preserving comments and notation.
pub fn format_source(source: &str) -> Result<String, Box<ParseError>> {
    let tokens = lex(source)?;
    let parsed = crate::parse_module_located(source).ok();
    let items = items(source, &tokens)?;
    let preferred = parsed
        .as_ref()
        .map(|parsed| conditional::rewrite(&items, &parsed.locations));
    let document = layout::module(preferred.as_deref().unwrap_or(&items));
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut result = document::render(&document, newline);
    let after = lex(&result)?;
    if tokens
        .iter()
        .map(|t| &t.kind)
        .ne(after.iter().map(|t| &t.kind))
        && parsed.as_ref().is_none_or(|before| {
            crate::parse_module(&result).map_or(true, |after| before.module != after)
        })
    {
        // Keep call syntax if custom notation would change the parsed expression.

        result = document::render(&layout::module(&items), newline);
        if tokens
            .iter()
            .map(|t| &t.kind)
            .ne(lex(&result)?.iter().map(|t| &t.kind))
        {
            return Err(crate::error::error(
                source,
                0,
                "formatting would change source tokens",
            ));
        }
    }
    Ok(result)
}

#[derive(Clone, Debug)]
struct Item<'a> {
    kind: ItemKind<'a>,
    start: usize,
    end: usize,
    space_before: bool,
}
#[derive(Clone, Debug)]
enum ItemKind<'a> {
    Conditional([Vec<Item<'a>>; 3]),
    Atom(&'a Kind, &'a str),
    Group {
        open: &'a str,
        close: &'a str,
        children: Vec<Item<'a>>,
    },
    Comment(&'a str, bool),
    Blank,
}
impl Item<'_> {
    fn symbol(&self, symbol: char) -> bool {
        matches!(self.kind, ItemKind::Atom(Kind::Symbol(c), _) if *c == symbol)
    }
    fn word(&self) -> Option<&str> {
        if let ItemKind::Atom(Kind::Ident(word), _) = &self.kind {
            Some(word)
        } else {
            None
        }
    }
    fn is_comment(&self) -> bool {
        matches!(self.kind, ItemKind::Comment(..))
    }
}

fn items<'a>(source: &'a str, tokens: &'a [Token]) -> Result<Vec<Item<'a>>, Box<ParseError>> {
    let mut flat = Vec::new();
    let mut previous_end = 0;
    for token in tokens {
        let mut start = previous_end;
        while start < token.offset {
            let rest = &source[start..token.offset];
            let whitespace = rest.len() - rest.trim_start().len();
            if rest[..whitespace].bytes().filter(|b| *b == b'\n').count() >= 2 {
                flat.push(Item {
                    kind: ItemKind::Blank,
                    start,
                    end: start + whitespace,
                    space_before: false,
                });
            }
            start += whitespace;
            if start == token.offset {
                break;
            }
            let end = start
                + source[start..token.offset]
                    .find('\n')
                    .unwrap_or(token.offset - start);
            let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
            let inline = !source[line_start..start].trim().is_empty();
            flat.push(Item {
                kind: ItemKind::Comment(source[start..end].trim_end(), inline),
                start,
                end,
                space_before: true,
            });
            start = end;
        }
        if token.kind != Kind::End {
            flat.push(Item {
                kind: ItemKind::Atom(&token.kind, &source[token.offset..token.end]),
                start: token.offset,
                end: token.end,
                space_before: token.offset > previous_end,
            });
        }
        previous_end = token.end;
    }
    let mut iter = flat.into_iter().peekable();
    tree(source, &mut iter, None, 0)
}

fn tree<'a>(
    source: &str,
    iter: &mut std::iter::Peekable<std::vec::IntoIter<Item<'a>>>,
    closing: Option<char>,
    depth: usize,
) -> Result<Vec<Item<'a>>, Box<ParseError>> {
    let mut result = Vec::new();
    while let Some(mut item) = iter.next() {
        if let ItemKind::Atom(Kind::Symbol(c), text) = item.kind {
            if Some(*c) == closing {
                result.push(item);
                return Ok(result);
            }
            let close = match c {
                '(' => Some(')'),
                '[' => Some(']'),
                '{' => Some('}'),
                _ => None,
            };
            if let Some(close) = close {
                if depth >= 256 {
                    return Err(crate::error::error(
                        source,
                        item.start,
                        "formatter nesting limit exceeded",
                    ));
                }
                let mut children = tree(source, iter, Some(close), depth + 1)?;
                let last = children.pop().ok_or_else(|| {
                    crate::error::error(source, item.start, "unclosed formatting delimiter")
                })?;
                let ItemKind::Atom(_, close_text) = last.kind else {
                    unreachable!()
                };
                item.end = last.end;
                item.kind = ItemKind::Group {
                    open: text,
                    close: close_text,
                    children,
                };
            } else if matches!(c, ')' | ']' | '}') {
                return Err(crate::error::error(
                    source,
                    item.start,
                    "unmatched formatting delimiter",
                ));
            }
        }
        result.push(item);
    }
    if closing.is_some() {
        return Err(crate::error::error(
            source,
            source.len(),
            "unclosed formatting delimiter",
        ));
    }
    Ok(result)
}
