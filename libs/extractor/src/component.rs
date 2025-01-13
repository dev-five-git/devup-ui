use crate::ExtractStyleValue::Static;
use crate::{ExtractStaticStyle, ExtractStyleValue};

/// devup-ui export variable kind
#[derive(Debug)]
pub enum ExportVariableKind {
    Box,
    Text,
    Button,
    Input,
    Flex,
    VStack,
    Center,
    Css,
}

impl ExportVariableKind {
    /// Convert the kind to a tag
    pub fn to_tag(&self) -> String {
        match self {
            ExportVariableKind::Center
            | ExportVariableKind::VStack
            | ExportVariableKind::Flex
            | ExportVariableKind::Box => "div",
            ExportVariableKind::Text => "span",
            ExportVariableKind::Button => "button",
            ExportVariableKind::Input => "input",
            ExportVariableKind::Css => unreachable!(),
        }
        .to_string()
    }
}

impl ExportVariableKind {
    pub fn extract(&self) -> Vec<ExtractStyleValue> {
        match self {
            ExportVariableKind::Input
            | ExportVariableKind::Button
            | ExportVariableKind::Css
            | ExportVariableKind::Text
            | ExportVariableKind::Box => vec![],
            ExportVariableKind::Flex => vec![Static(ExtractStaticStyle {
                value: "flex".to_string(),
                property: "display".to_string(),
                level: 0,
            })],
            ExportVariableKind::VStack => {
                vec![
                    Static(ExtractStaticStyle {
                        value: "flex".to_string(),
                        property: "display".to_string(),
                        level: 0,
                    }),
                    Static(ExtractStaticStyle {
                        value: "column".to_string(),
                        property: "flexDirection".to_string(),
                        level: 0,
                    }),
                ]
            }
            ExportVariableKind::Center => {
                vec![
                    Static(ExtractStaticStyle {
                        value: "flex".to_string(),
                        property: "display".to_string(),
                        level: 0,
                    }),
                    Static(ExtractStaticStyle {
                        value: "center".to_string(),
                        property: "justifyContent".to_string(),
                        level: 0,
                    }),
                    Static(ExtractStaticStyle {
                        value: "center".to_string(),
                        property: "alignItems".to_string(),
                        level: 0,
                    }),
                ]
            }
        }
    }
}

impl From<String> for ExportVariableKind {
    fn from(kind: String) -> Self {
        match kind.as_str() {
            "Box" => ExportVariableKind::Box,
            "Text" => ExportVariableKind::Text,
            "Button" => ExportVariableKind::Button,
            "Input" => ExportVariableKind::Input,
            "Flex" => ExportVariableKind::Flex,
            "VStack" => ExportVariableKind::VStack,
            "Center" => ExportVariableKind::Center,
            "css" => ExportVariableKind::Css,
            _ => panic!("Unknown component kind: {}", kind),
        }
    }
}
