mod conditional;
mod document;
mod layout;
use crate::{ParseError, lexer::{Kind, Token, lex}};
use document::Doc;

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

}

