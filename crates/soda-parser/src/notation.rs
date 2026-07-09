use crate::{Kind, ParseError, Parser, Token, lex};
use soda_ast::{Module, NotationDecl};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Hole(usize),
    Literal(Kind),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Notation {
    pub judgment: String,
    pub arity: usize,
    pub parts: Vec<Part>,
}
impl Notation {

}

impl Parser<'_> {

}
