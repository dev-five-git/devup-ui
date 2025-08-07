use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractFontFace {
    pub file: String,
    pub properties: BTreeMap<String, String>,
}
