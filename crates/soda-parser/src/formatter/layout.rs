use super::{Doc, Item, ItemKind};
use crate::lexer::Kind;

struct Family<'a> {
    keyword: &'a str,
    name: Option<&'a str>,
    subject: Option<&'a str>,
    defined_function: bool,
}

