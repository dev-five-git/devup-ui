use crate::class_arguments::ClassCall;

#[derive(Debug, PartialEq, Eq)]
pub enum UtilType {
    Css,
    /// Emotion's `cx`: class names composed like `css()` parts
    Cx,
    /// Emotion's `merge`: the classes of a class string, composed like `cx`
    Merge,
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
            "cx" => Some(UtilType::Cx),
            "merge" => Some(UtilType::Merge),
            "globalCss" => Some(UtilType::GlobalCss),
            "createGlobalStyle" => Some(UtilType::GlobalCssComponent),
            "keyframes" => Some(UtilType::Keyframes),
            _ => None,
        }
    }

    /// Whether the call gives a class name composed as `css()` composes its parts
    #[must_use]
    pub const fn is_css(&self) -> bool {
        matches!(self, UtilType::Css | UtilType::Cx | UtilType::Merge)
    }

    /// The Emotion class function the call is, if it is one
    #[must_use]
    pub const fn class_call(&self) -> Option<ClassCall> {
        match self {
            UtilType::Cx => Some(ClassCall::Cx),
            UtilType::Merge => Some(ClassCall::Merge),
            _ => None,
        }
    }

    /// The name the call is reported under
    #[must_use]
    pub const fn api(&self) -> &'static str {
        match self {
            UtilType::Css => "css",
            UtilType::Cx => "cx",
            UtilType::Merge => "merge",
            UtilType::Keyframes => "keyframes",
            UtilType::GlobalCss | UtilType::GlobalCssComponent => "globalCss",
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
