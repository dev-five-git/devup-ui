use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::theme::{Theme, Typographies, Typography};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Only the seven raw fields of a used preset frame, never a theme registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct FrozenTypography {
    pub font_family: Option<String>,
    pub font_size: Option<String>,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
    pub line_height: Option<String>,
    pub letter_spacing: Option<String>,
    pub text_transform: Option<String>,
}

impl From<&Typography> for FrozenTypography {
    fn from(frame: &Typography) -> Self {
        Self {
            font_family: frame.font_family.clone(),
            font_size: frame.font_size.clone(),
            font_style: frame.font_style.clone(),
            font_weight: frame.font_weight.clone(),
            line_height: frame.line_height.clone(),
            letter_spacing: frame.letter_spacing.clone(),
            text_transform: frame.text_transform.clone(),
        }
    }
}

impl FrozenTypography {
    fn thaw(&self) -> Typography {
        Typography {
            font_family: self.font_family.clone(),
            font_size: self.font_size.clone(),
            font_style: self.font_style.clone(),
            font_weight: self.font_weight.clone(),
            line_height: self.line_height.clone(),
            letter_spacing: self.letter_spacing.clone(),
            text_transform: self.text_transform.clone(),
        }
    }
}

/// Raw frames for precisely one named preset; sparse positions are retained.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrozenPreset {
    pub name: String,
    pub frames: Vec<Option<FrozenTypography>>,
}

/// A tolerated authored yield, parsed with the existing preset key grammar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Yield {
    pub property: String,
    pub from: u8,
}

/// Parse yields without tightening the existing ignored-malformed-item policy.
pub fn preset_key(value: &str) -> (&str, Vec<Yield>) {
    let (preset, yielded) = value.split_once('|').unwrap_or((value, ""));
    let yields = yielded
        .split(',')
        .filter_map(|item| {
            let (property, from) = item.split_once(':')?;
            Some(Yield {
                property: property.into(),
                from: from.parse().ok()?,
            })
        })
        .collect();
    (preset, yields)
}

impl FrozenPreset {
    /// Replay `Theme::typography_declarations` then `content_typography`'s yields.
    pub fn declarations(&self, start: u8, yielded: &[Yield]) -> Vec<(u8, String, String)> {
        let frames = self
            .frames
            .iter()
            .map(|frame| frame.as_ref().map(FrozenTypography::thaw))
            .collect();
        // A local helper receiver contains only frozen frames, not live theme authority.
        let theme = Theme {
            typography: BTreeMap::from([(self.name.clone(), Typographies(frames))]),
            ..Theme::default()
        };
        let mut result: Vec<_> = theme
            .typography_declarations(&self.name, start)
            .into_iter()
            .map(|(level, property, value)| (level, property.to_string(), value))
            .collect();
        result.retain(|(level, property, _)| {
            !yielded
                .iter()
                .any(|item| item.property == *property && *level >= item.from)
        });
        result
    }

    /// Exact current effective typography value used inside keyframe members.
    pub fn identity(&self, start: u8, yielded: &[Yield]) -> String {
        let declarations = self.declarations(start, yielded);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            &u64::try_from(declarations.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for (level, property, value) in declarations {
            bytes.push(level);
            for field in [property, value] {
                bytes.extend_from_slice(
                    &u64::try_from(field.len()).unwrap_or(u64::MAX).to_be_bytes(),
                );
                bytes.extend_from_slice(field.as_bytes());
            }
        }
        let mut identity = String::from("t");
        for byte in bytes {
            identity.push(char::from(HEX[usize::from(byte >> 4)]));
            identity.push(char::from(HEX[usize::from(byte & 15)]));
        }
        identity
    }
}
