#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DomainErrorKind {
    #[default]
    InvalidDefinition,
    NonTotalMap,
    NonMonotoneMap,
}

pub type DomainError = soda_ast::DefinitionError<DomainErrorKind>;
