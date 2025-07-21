use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

use crate::extract_style::{
    extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
};

/// devup-ui export variable kind
#[derive(Debug, PartialEq, Clone, EnumIter, Display)]
pub enum ExportVariableKind {
    Box,
    Text,
    Button,
    Input,
    Flex,
    VStack,
    Center,
    Image,
    Grid,
}

impl ExportVariableKind {
    /// Convert the kind to a tag
    pub fn to_tag(&self) -> Result<&str, &str> {
        match self {
            ExportVariableKind::Center
            | ExportVariableKind::VStack
            | ExportVariableKind::Grid
            | ExportVariableKind::Flex
            | ExportVariableKind::Box => Ok("div"),
            ExportVariableKind::Text => Ok("span"),
            ExportVariableKind::Image => Ok("img"),
            ExportVariableKind::Button => Ok("button"),
            ExportVariableKind::Input => Ok("input"),
        }
    }
}

impl ExportVariableKind {
    pub fn extract(&self) -> Vec<ExtractStyleValue> {
        match self {
            ExportVariableKind::Input
            | ExportVariableKind::Button
            | ExportVariableKind::Text
            | ExportVariableKind::Image
            | ExportVariableKind::Box => vec![],
            ExportVariableKind::Flex => {
                vec![ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "display", "flex", 0, None,
                ))]
            }
            ExportVariableKind::VStack => {
                vec![
                    ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                        "display", "flex", 0, None,
                    )),
                    ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                        "flexDirection",
                        "column",
                        0,
                        None,
                    )),
                ]
            }
            ExportVariableKind::Center => {
                vec![
                    ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                        "display", "flex", 0, None,
                    )),
                    ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                        "justifyContent",
                        "center",
                        0,
                        None,
                    )),
                    ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                        "alignItems",
                        "center",
                        0,
                        None,
                    )),
                ]
            }
            ExportVariableKind::Grid => {
                vec![ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "display", "grid", 0, None,
                ))]
            }
        }
    }
}

impl TryFrom<String> for ExportVariableKind {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        for kind in ExportVariableKind::iter() {
            if kind.to_string() == value {
                return Ok(kind);
            }
        }
        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_from_export_variable() {
        assert_eq!(
            ExportVariableKind::try_from("Box".to_string()),
            Ok(ExportVariableKind::Box)
        );
        assert_eq!(
            ExportVariableKind::try_from("Text".to_string()),
            Ok(ExportVariableKind::Text)
        );
        assert_eq!(
            ExportVariableKind::try_from("Image".to_string()),
            Ok(ExportVariableKind::Image)
        );
        assert_eq!(
            ExportVariableKind::try_from("Button".to_string()),
            Ok(ExportVariableKind::Button)
        );
        assert_eq!(
            ExportVariableKind::try_from("Input".to_string()),
            Ok(ExportVariableKind::Input)
        );
        assert_eq!(
            ExportVariableKind::try_from("Flex".to_string()),
            Ok(ExportVariableKind::Flex)
        );
        assert_eq!(
            ExportVariableKind::try_from("VStack".to_string()),
            Ok(ExportVariableKind::VStack)
        );
        assert_eq!(
            ExportVariableKind::try_from("Center".to_string()),
            Ok(ExportVariableKind::Center)
        );
        assert_eq!(
            ExportVariableKind::try_from("Grid".to_string()),
            Ok(ExportVariableKind::Grid)
        );
        assert!(ExportVariableKind::try_from("css".to_string()).is_err());
        assert!(ExportVariableKind::try_from("foo".to_string()).is_err());
    }

    #[test]
    fn test_to_tag() {
        assert_eq!(ExportVariableKind::Box.to_tag(), Ok("div"));
        assert_eq!(ExportVariableKind::Text.to_tag(), Ok("span"));
        assert_eq!(ExportVariableKind::Image.to_tag(), Ok("img"));
        assert_eq!(ExportVariableKind::Button.to_tag(), Ok("button"));
        assert_eq!(ExportVariableKind::Input.to_tag(), Ok("input"));
        assert_eq!(ExportVariableKind::Flex.to_tag(), Ok("div"));
        assert_eq!(ExportVariableKind::VStack.to_tag(), Ok("div"));
        assert_eq!(ExportVariableKind::Center.to_tag(), Ok("div"));
        assert_eq!(ExportVariableKind::Grid.to_tag(), Ok("div"));
    }

    #[test]
    fn test_extract_style_from_kind() {
        assert_eq!(ExportVariableKind::Box.extract(), vec![]);
        assert_eq!(ExportVariableKind::Text.extract(), vec![]);
        assert_eq!(ExportVariableKind::Image.extract(), vec![]);
        assert_eq!(ExportVariableKind::Button.extract(), vec![]);
        assert_eq!(ExportVariableKind::Input.extract(), vec![]);
        assert_eq!(
            ExportVariableKind::Flex.extract(),
            vec![ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                "display", "flex", 0, None,
            ))]
        );
        assert_eq!(
            ExportVariableKind::VStack.extract(),
            vec![
                ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "display", "flex", 0, None,
                )),
                ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "flexDirection",
                    "column",
                    0,
                    None,
                ))
            ]
        );
        assert_eq!(
            ExportVariableKind::Center.extract(),
            vec![
                ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "display", "flex", 0, None,
                )),
                ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "justifyContent",
                    "center",
                    0,
                    None,
                )),
                ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                    "alignItems",
                    "center",
                    0,
                    None,
                ))
            ]
        );
        assert_eq!(
            ExportVariableKind::Grid.extract(),
            vec![ExtractStyleValue::Static(ExtractStaticStyle::new_basic(
                "display", "grid", 0, None,
            ))]
        );
    }
}
