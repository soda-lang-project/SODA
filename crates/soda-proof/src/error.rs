#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InductionErrorKind {
    #[default]
    InvalidDefinition,
    IncompleteCases,
    UnanchoredHypothesis,
}

pub type InductionError = soda_ast::DefinitionError<InductionErrorKind>;
