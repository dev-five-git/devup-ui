#[derive(Debug, PartialEq, Eq)]
pub enum UtilType {
    Css,
    GlobalCss,
    Keyframes,
}

impl UtilType {
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "css" => Some(UtilType::Css),
            "globalCss" => Some(UtilType::GlobalCss),
            "keyframes" => Some(UtilType::Keyframes),
            _ => None,
        }
    }
}
