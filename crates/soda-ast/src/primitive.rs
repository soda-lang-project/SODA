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
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "apply" => Self::Apply,
            "subst" => Self::Substitute,
            "alpha_eq" => Self::AlphaEq,
            "fresh" => Self::Fresh,
            "add" => Self::Add,
            "sub" => Self::Subtract,
            "mul" => Self::Multiply,
            "div" => Self::Divide,
            "mod" => Self::Remainder,
            "eq" => Self::Equal,
            "lt" => Self::Less,
            "le" => Self::LessEqual,
            "gt" => Self::Greater,
            "ge" => Self::GreaterEqual,
            "and" => Self::And,
            "or" => Self::Or,
            "not" => Self::Not,
            "if" => Self::If,
            _ => return None,
        })
    }

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

    pub const fn arity(self) -> usize {
        match self {
            Self::Not => 1,
            Self::If | Self::Substitute => 3,
            Self::Apply
            | Self::AlphaEq
            | Self::Fresh
            | Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Remainder
            | Self::Equal
            | Self::Less
            | Self::LessEqual
            | Self::Greater
            | Self::GreaterEqual
            | Self::And
            | Self::Or => 2,
        }
    }
}
