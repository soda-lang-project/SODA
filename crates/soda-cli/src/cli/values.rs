use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum ReductionOrder {
    #[default]
    Outermost,
    Innermost,
}

impl From<ReductionOrder> for soda_semantics::Strategy {
    fn from(value: ReductionOrder) -> Self {
        match value {
            ReductionOrder::Outermost => Self::Outermost,
            ReductionOrder::Innermost => Self::Innermost,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum Notation {
    #[default]
    Canonical,
    Unicode,
    Ascii,
}

impl From<Notation> for soda_parser::NotationStyle {
    fn from(value: Notation) -> Self {
        match value {
            Notation::Canonical => Self::Canonical,
            Notation::Unicode => Self::Unicode,
            Notation::Ascii => Self::Ascii,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum DrawingStyle {
    Unicode,
    #[default]
    Ascii,
}

impl From<DrawingStyle> for soda_diagnostics::Style {
    fn from(value: DrawingStyle) -> Self {
        match value {
            DrawingStyle::Unicode => Self::Unicode,
            DrawingStyle::Ascii => Self::Ascii,
        }
    }
}
