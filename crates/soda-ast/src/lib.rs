mod bindings;
mod declarations;
mod error;
pub use error::DefinitionError;
mod matching;
mod primitive;
mod term;
pub use declarations::{Clause, ClauseDetail, ComputedBinding, DomainDecl, DomainExpr, Equation, InductionCase, InductionProof, JudgmentDecl, JudgmentKind, Module, MonadDecl, NotationDecl, Proof, ProofStep, RecordDecl, Rule, SemanticMap, Signature, SignatureKind, SortDecl};

pub use primitive::Primitive;
use std::collections::BTreeMap;
pub use term::Term;
pub type Substitution = BTreeMap<String, Term>;
