use crate::{RuntimeError, RuntimeErrorKind};
use soda_ast::{Primitive, Term};

pub(crate) enum Builtin {
    Unknown,
    Pending,
    Value(Term),
    Error(RuntimeError),
}

