#[derive(Debug)]
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
