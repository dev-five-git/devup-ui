use super::{
    EvidenceError,
    raw::{Object, Raw},
    wire::{Wire, tagged, variant, wire_struct},
};
use crate::StyleSheetProperty;
use css::style_selector::{AtRule, AtRuleKind, StyleSelector};

impl Wire for AtRuleKind {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        match String::read(raw)?.as_str() {
            "Media" => Ok(Self::Media),
            "Supports" => Ok(Self::Supports),
            "Container" => Ok(Self::Container),
            "Layer" => Ok(Self::Layer),
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Media => "Media",
            Self::Supports => "Supports",
            Self::Container => "Container",
            Self::Layer => "Layer",
        }
        .to_string()
        .write()
    }
}
wire_struct!(AtRule { kind => "kind", query => "query" });
wire_struct!(StyleSheetProperty {
    class_name => "c", property => "p", value => "v", selector => "s",
    layer => "l", typography => "t", hoisted => "h", owner_reset => "r"
});

impl Wire for StyleSelector {
    fn read(raw: Raw) -> Result<Self, EvidenceError> {
        let (name, payload) = variant(raw)?;
        match name.as_str() {
            "Selector" => Ok(Self::Selector(String::read(payload)?)),
            "Global" => {
                let (selector, owner) = <(String, String)>::read(payload)?;
                Ok(Self::Global(selector, owner))
            }
            "At" => {
                let mut object = Object::read(payload)?;
                let value = Self::At {
                    kind: object.take("kind")?,
                    query: object.take("query")?,
                    selector: object.take("selector")?,
                    outer: object.take("outer")?,
                    file: object.take("file")?,
                };
                object.finish()?;
                Ok(value)
            }
            _ => Err(EvidenceError::Schema),
        }
    }
    fn write(&self) -> Raw {
        match self {
            Self::Selector(value) => tagged("Selector", value.write()),
            Self::Global(selector, file) => {
                tagged("Global", Raw::Array(vec![selector.write(), file.write()]))
            }
            Self::At {
                kind,
                query,
                selector,
                outer,
                file,
            } => tagged(
                "At",
                Raw::Object(Object(vec![
                    ("kind".into(), kind.write()),
                    ("query".into(), query.write()),
                    ("selector".into(), selector.write()),
                    ("outer".into(), outer.write()),
                    ("file".into(), file.write()),
                ])),
            ),
        }
    }
}
