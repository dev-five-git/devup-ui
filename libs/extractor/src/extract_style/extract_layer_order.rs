/// The cascade layers a file puts styles in, in the order the file first uses them
#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractLayerOrder {
    pub file: String,
    pub layers: Vec<String>,
}
