use super::{Item, ItemKind};
use crate::{Locations, lexer::Kind};
use soda_ast::Term;
use soda_diagnostics::Span;
use std::{cmp::Reverse, collections::BTreeMap};

