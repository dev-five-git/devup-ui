#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractImport {
    /// import must be global css
    pub url: String,
    pub file: String,
}
