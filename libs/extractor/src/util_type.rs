#[derive(Debug, PartialEq, Eq)]
pub enum UtilType {
    Css,
    GlobalCss,
    /// `createGlobalStyle` — global CSS that callers render as a component
    /// (`<GlobalStyle />`), so the call must collapse to a component rather than
    /// to the empty statement `globalCss` becomes.
    GlobalCssComponent,
    Keyframes,
}

impl UtilType {
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "css" => Some(UtilType::Css),
            "globalCss" => Some(UtilType::GlobalCss),
            "createGlobalStyle" => Some(UtilType::GlobalCssComponent),
            "keyframes" => Some(UtilType::Keyframes),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_component(&self) -> bool {
        matches!(self, UtilType::GlobalCssComponent)
    }

    #[must_use]
    pub const fn is_global(&self) -> bool {
        matches!(self, UtilType::GlobalCss | UtilType::GlobalCssComponent)
    }
}
