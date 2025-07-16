#[derive(Debug, PartialEq)]
pub enum UtilType {
    Css,
    GlobalCss,
    Keyframes,
}

impl TryFrom<String> for UtilType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "css" {
            Ok(UtilType::Css)
        } else if value == "globalCss" {
            Ok(UtilType::GlobalCss)
        } else if value == "keyframes" {
            Ok(UtilType::Keyframes)
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
    #[case("css".to_string(), Ok(UtilType::Css))]
    #[case("globalCss".to_string(), Ok(UtilType::GlobalCss))]
    #[case("keyframes".to_string(), Ok(UtilType::Keyframes))]
    #[case("unknown".to_string(), Err("unknown".to_string()))]
    #[case("".to_string(), Err("".to_string()))]
    fn test_util_type_try_from(#[case] input: String, #[case] expected: Result<UtilType, String>) {
        assert_eq!(UtilType::try_from(input), expected);
    }
}
