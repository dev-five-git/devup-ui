#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractCss {
    /// css must be global css
    pub css: String,
    pub file: String,
}
