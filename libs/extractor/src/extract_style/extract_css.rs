#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractCss {
    pub file: String,
    /// Where the rules are written in the file; the sheet keeps them in this order
    pub order: u32,
    /// css must be global css
    pub css: String,
}
