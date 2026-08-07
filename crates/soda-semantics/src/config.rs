pub const MAX_DEPTH: usize = 256;
pub const DEFAULT_FUEL: usize = 10_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Strategy {
    #[default]
    Outermost,
    Innermost,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub fuel: usize,
    /// Requested structural/guard depth; clamped to [`MAX_DEPTH`].
    pub max_depth: usize,
    pub strategy: Strategy,
    pub trace: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fuel: DEFAULT_FUEL,
            max_depth: MAX_DEPTH,
            strategy: Strategy::Outermost,
            trace: false,
        }
    }
}

impl Config {
    pub(crate) fn bounded(&self) -> Self {
        Self {
            max_depth: self.max_depth.min(MAX_DEPTH),
            ..self.clone()
        }
    }
}
