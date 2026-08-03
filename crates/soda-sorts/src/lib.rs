mod error;
mod inference;
mod proofs;
mod schema;
mod sort;
mod validation;
pub const MAX_SORT_DEPTH: usize = 256;
pub use error::{SortError, SortErrorKind};
pub use schema::TypeChecker;
use soda_ast::{Module, Term};
pub use sort::Sort;
use validation::bindings_require_sort_check;
pub use validation::{TypeReport, requires_sort_check};

pub fn check_term(module: &Module, term: &Term) -> Result<Sort, SortError> {
    TypeChecker::new(module)?.check_term(term)
}
