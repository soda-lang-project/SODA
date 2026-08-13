//! Equality certificates are checked relative to the module's axioms.
mod analysis;
mod error;
mod kernel;
mod overlaps;
mod validation;

pub use analysis::{TerminationAnalysis};
pub use error::{VerificationError, VerificationErrorKind};
pub use kernel::{VerificationReport, VerifiedProof};
pub use overlaps::{CriticalPair, CriticalPairAnalysis};
pub use validation::{MAX_VALIDATION_DEPTH, validate_module};
