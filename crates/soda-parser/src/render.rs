use crate::{Kind, Notation, Parser, Part};
use soda_ast::{Module, Term};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NotationStyle {
    #[default]
    Canonical,
    Unicode,
    Ascii,
}

struct Renderer {
    notations: Vec<Notation>,
}

impl Renderer {

}
