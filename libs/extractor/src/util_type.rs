#[derive(Debug, PartialEq, Eq)]
pub enum UtilType {
    Css,
    GlobalCss,
    Keyframes,
}

impl TryFrom<String> for UtilType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        UtilType::from_str_opt(&value).ok_or(value)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("css".to_string(), Ok(UtilType::Css))]
    #[case("globalCss".to_string(), Ok(UtilType::GlobalCss))]
    #[case("keyframes".to_string(), Ok(UtilType::Keyframes))]
    #[case("unknown".to_string(), Err("unknown".to_string()))]
    #[case(String::new(), Err(String::new()))]
    fn test_util_type_try_from(#[case] input: String, #[case] expected: Result<UtilType, String>) {
        assert_eq!(UtilType::try_from(input), expected);
    }
}
