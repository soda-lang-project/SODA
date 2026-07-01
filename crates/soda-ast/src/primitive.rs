#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Primitive {
    Apply,
    Substitute,
    AlphaEq,
    Fresh,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    If,
}

impl Primitive {
    pub const ALL: &[Self] = &[
        Self::Apply,
        Self::Substitute,
        Self::AlphaEq,
        Self::Fresh,
        Self::Add,
        Self::Subtract,
        Self::Multiply,
        Self::Divide,
        Self::Remainder,
        Self::Equal,
        Self::Less,
        Self::LessEqual,
        Self::Greater,
        Self::GreaterEqual,
        Self::And,
        Self::Or,
        Self::Not,
        Self::If,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Substitute => "subst",
            Self::AlphaEq => "alpha_eq",
            Self::Fresh => "fresh",
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "mod",
            Self::Equal => "eq",
            Self::Less => "lt",
            Self::LessEqual => "le",
            Self::Greater => "gt",
            Self::GreaterEqual => "ge",
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
            Self::If => "if",
        }
    }

}
