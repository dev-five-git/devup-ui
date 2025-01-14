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
            "Button" => Ok(ExportVariableKind::Button),
            "Input" => Ok(ExportVariableKind::Input),
            "Flex" => Ok(ExportVariableKind::Flex),
            "VStack" => Ok(ExportVariableKind::VStack),
            "Center" => Ok(ExportVariableKind::Center),
            "css" => Ok(ExportVariableKind::Css),
            _ => Err(()),
        }
    }
}
