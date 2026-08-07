//! Rules run in declaration order; guards share the caller's reduction budget.
//! Arithmetic uses checked i64 operations, and `if` evaluates only its selected branch.
mod builtin;
mod engine;

mod config;
mod error;
mod result;
pub use config::{Config, DEFAULT_FUEL, MAX_DEPTH, Strategy};
pub use engine::Engine;
pub use error::{RuntimeError, RuntimeErrorKind};
pub use result::{Evaluation, Limit, Status, TraceStep, ValidationError};
