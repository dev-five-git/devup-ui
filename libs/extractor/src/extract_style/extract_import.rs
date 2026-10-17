#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractImport {
    pub file: String,
    /// Where the import is written in the file; the sheet keeps them in this order
    pub order: u32,
    /// import must be global css
    pub url: String,
}
