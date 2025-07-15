#[derive(Debug, PartialEq)]
pub enum CssType {
    Css,
    GlobalCss,
}

impl TryFrom<String> for CssType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "css" {
            Ok(CssType::Css)
        } else if value == "globalCss" {
            Ok(CssType::GlobalCss)
        } else {
            Err(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("css".to_string(), Ok(CssType::Css))]
    #[case("globalCss".to_string(), Ok(CssType::GlobalCss))]
    #[case("unknown".to_string(), Err("unknown".to_string()))]
    #[case("".to_string(), Err("".to_string()))]
    fn test_css_type_try_from(#[case] input: String, #[case] expected: Result<CssType, String>) {
        assert_eq!(CssType::try_from(input), expected);
    }
}
