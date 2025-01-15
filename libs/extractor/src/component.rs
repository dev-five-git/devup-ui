use crate::ExtractStyleValue::Static;
use crate::{ExtractStaticStyle, ExtractStyleValue};

/// devup-ui export variable kind
#[derive(Debug, PartialEq, Clone)]
pub enum ExportVariableKind {
    Box,
    Text,
    Button,
    Input,
    Flex,
    VStack,
    Center,
    Image,
}

impl ExportVariableKind {
    /// Convert the kind to a tag
    pub fn to_tag(&self) -> Result<&str, &str> {
        match self {
            ExportVariableKind::Center
            | ExportVariableKind::VStack
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
            ExportVariableKind::Flex => vec![Static(ExtractStaticStyle {
                value: "flex".to_string(),
                property: "display".to_string(),
                level: 0,
                selector: None,
            })],
            ExportVariableKind::VStack => {
                vec![
                    Static(ExtractStaticStyle {
                        value: "flex".to_string(),
                        property: "display".to_string(),
                        level: 0,
                        selector: None,
                    }),
                    Static(ExtractStaticStyle {
                        value: "column".to_string(),
                        property: "flexDirection".to_string(),
                        level: 0,
                        selector: None,
                    }),
                ]
            }
            ExportVariableKind::Center => {
                vec![
                    Static(ExtractStaticStyle {
                        value: "flex".to_string(),
                        property: "display".to_string(),
                        level: 0,
                        selector: None,
                    }),
                    Static(ExtractStaticStyle {
                        value: "center".to_string(),
                        property: "justifyContent".to_string(),
                        level: 0,
                        selector: None,
                    }),
                    Static(ExtractStaticStyle {
                        value: "center".to_string(),
                        property: "alignItems".to_string(),
                        level: 0,
                        selector: None,
                    }),
                ]
            }
        }
    }
}

impl TryFrom<String> for ExportVariableKind {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Box" => Ok(ExportVariableKind::Box),
            "Text" => Ok(ExportVariableKind::Text),
            "Image" => Ok(ExportVariableKind::Image),
            "Button" => Ok(ExportVariableKind::Button),
            "Input" => Ok(ExportVariableKind::Input),
            "Flex" => Ok(ExportVariableKind::Flex),
            "VStack" => Ok(ExportVariableKind::VStack),
            "Center" => Ok(ExportVariableKind::Center),
            _ => Err(()),
        }
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
            vec![Static(ExtractStaticStyle {
                value: "flex".to_string(),
                property: "display".to_string(),
                level: 0,
                selector: None,
            })]
        );
        assert_eq!(
            ExportVariableKind::VStack.extract(),
            vec![
                Static(ExtractStaticStyle {
                    value: "flex".to_string(),
                    property: "display".to_string(),
                    level: 0,
                    selector: None,
                }),
                Static(ExtractStaticStyle {
                    value: "column".to_string(),
                    property: "flexDirection".to_string(),
                    level: 0,
                    selector: None,
                })
            ]
        );
        assert_eq!(
            ExportVariableKind::Center.extract(),
            vec![
                Static(ExtractStaticStyle {
                    value: "flex".to_string(),
                    property: "display".to_string(),
                    level: 0,
                    selector: None,
                }),
                Static(ExtractStaticStyle {
                    value: "center".to_string(),
                    property: "justifyContent".to_string(),
                    level: 0,
                    selector: None,
                }),
                Static(ExtractStaticStyle {
                    value: "center".to_string(),
                    property: "alignItems".to_string(),
                    level: 0,
                    selector: None,
                })
            ]
        );
    }
}
