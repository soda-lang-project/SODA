//! Equality certificates are checked relative to the module's axioms.
mod analysis;
mod error;
mod kernel;
mod overlaps;
mod validation;

pub use analysis::{TerminationAnalysis, analyze_termination, unify};
pub use error::{VerificationError, VerificationErrorKind};
pub use kernel::{VerificationReport, VerifiedProof, verify_module, verify_proof};
pub use overlaps::{CriticalPair, CriticalPairAnalysis, critical_pairs};
pub use validation::{MAX_VALIDATION_DEPTH, validate_module};
