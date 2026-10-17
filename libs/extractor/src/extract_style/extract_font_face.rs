use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractFontFace {
    pub file: String,
    /// Where the rule is written in the file; the sheet keeps them in this order
    pub order: u32,
    pub properties: BTreeMap<String, String>,
}
