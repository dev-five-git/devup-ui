use std::collections::BTreeMap;

use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value},
    style_selector::StyleSelector,
};
use extractor::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use serde::{Deserialize, Serialize};

use crate::{
    StyleSheetProperty,
    counter_evidence::{Expansion, RecordFootprint, ReplayError},
};

#[path = "emission_seed_typography.rs"]
pub mod typography;
pub use typography::{FrozenPreset, FrozenTypography, Yield, preset_key};

/// Frozen delivery decision, supplied before insertion without resolver reads.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmissionContext {
    pub source_file: String,
    pub bucket: String,
    pub single_css: bool,
    pub hoisted: bool,
}

/// Serializable projection of the static token-resolution policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Resolution {
    CssVariable,
    FirstValue,
}

/// Independent normalized pre-emission inputs, including keyframe preset input.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeclarationSeed {
    pub property: String,
    pub value: String,
    pub level: u8,
    pub selector: Option<StyleSelector>,
    pub style_order: Option<u8>,
    pub layer: Option<String>,
    pub resolution: Resolution,
    pub first_value: Option<String>,
    pub preset: Option<FrozenPreset>,
}

impl DeclarationSeed {
    /// Capture supplied lookups with normalized IR, never from emitted records.
    pub fn capture(
        style: &ExtractStaticStyle,
        first_value: Option<String>,
        preset: Option<FrozenPreset>,
    ) -> Self {
        Self {
            property: style.property.clone(),
            value: style.value.clone(),
            level: style.level,
            selector: style.selector.clone(),
            style_order: style.style_order,
            layer: style.layer.clone(),
            resolution: match style.theme_token_resolution {
                ThemeTokenResolution::CssVariable => Resolution::CssVariable,
                ThemeTokenResolution::FirstValue => Resolution::FirstValue,
            },
            first_value,
            preset,
        }
    }

    /// Replay static normalization with frozen first-value/preset authority only.
    pub fn effective_value(&self) -> Result<String, ReplayError> {
        if self.property == "typography" {
            let (preset, yields) = self.typography()?;
            return Ok(preset.identity(self.level, &yields));
        }
        let value = match self.resolution {
            Resolution::CssVariable => self.value.as_str(),
            Resolution::FirstValue => self.first_value.as_deref().unwrap_or(&self.value),
        };
        let value = if self.property != "content" && check_multi_css_optimize(&self.property) {
            optimize_multi_css_value(value)
        } else {
            std::borrow::Cow::Borrowed(value)
        };
        Ok(css::content_value::emitted(&value).into_owned())
    }

    fn typography(&self) -> Result<(&FrozenPreset, Vec<Yield>), ReplayError> {
        let (name, yielded) = preset_key(&self.value);
        let preset = self.preset.as_ref().ok_or(ReplayError::MissingPreset)?;
        if self.property != "typography" || name != preset.name {
            return Err(ReplayError::Preset);
        }
        Ok((preset, yielded))
    }

    fn record(&self, context: &EmissionContext, name: &str, value: String) -> RecordFootprint {
        RecordFootprint::Property {
            bucket: context.bucket.clone(),
            order: self.style_order.unwrap_or(255),
            level: self.level,
            record: StyleSheetProperty {
                class_name: name.into(),
                property: self.property.clone(),
                value,
                selector: self.selector.clone(),
                layer: self.layer.clone(),
                typography: false,
                hoisted: context.hoisted,
                owner_reset: false,
            },
        }
    }
}

/// Original numeric site captured independently of delivery numbering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NumericSite {
    pub source: u32,
    pub at: usize,
    pub role: usize,
}

/// Replay inputs; variable allocation/site spelling is a future producer seam.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmissionInput {
    Static(DeclarationSeed),
    Dynamic {
        declaration: DeclarationSeed,
        variable: String,
        site: Option<NumericSite>,
        important: bool,
    },
    Typography(DeclarationSeed),
    Keyframes {
        steps: BTreeMap<String, Vec<DeclarationSeed>>,
    },
}

/// Independent inputs for a complete expansion, not copied sheet output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmissionSeed {
    pub placement: EmissionContext,
    pub body: EmissionInput,
}

impl EmissionSeed {
    /// Replay using the caller-supplied name, with no allocation or registry lookup.
    pub fn replay(&self, name: &str) -> Result<Expansion, ReplayError> {
        match &self.body {
            EmissionInput::Static(declaration) => Ok(Expansion::Static(vec![declaration.record(
                &self.placement,
                name,
                declaration.effective_value()?,
            )])),
            EmissionInput::Dynamic {
                declaration,
                variable,
                site,
                important,
            } => {
                let value = format!(
                    "var({variable}){}",
                    if *important { " !important" } else { "" }
                );
                let consumer = declaration.record(&self.placement, name, value);
                let reset = RecordFootprint::Property {
                    bucket: self.placement.bucket.clone(),
                    order: declaration.style_order.unwrap_or(255),
                    level: 0,
                    record: StyleSheetProperty {
                        class_name: name.into(),
                        property: variable.clone(),
                        value: "initial".into(),
                        selector: None,
                        layer: None,
                        typography: false,
                        hoisted: self.placement.hoisted,
                        owner_reset: true,
                    },
                };
                Ok(Expansion::Dynamic {
                    variable: variable.clone(),
                    site: site.clone(),
                    important: *important,
                    consumer: Box::new(consumer),
                    reset: Box::new(reset),
                })
            }
            EmissionInput::Typography(declaration) => {
                let (preset, yielded) = declaration.typography()?;
                let members = preset
                    .declarations(declaration.level, &yielded)
                    .into_iter()
                    .map(|(level, property, value)| RecordFootprint::Property {
                        bucket: self.placement.bucket.clone(),
                        order: declaration.style_order.unwrap_or(255),
                        level,
                        record: StyleSheetProperty {
                            class_name: name.into(),
                            property,
                            value,
                            selector: declaration.selector.clone(),
                            layer: Some(
                                declaration
                                    .layer
                                    .as_ref()
                                    .map_or_else(|| "t".into(), |layer| format!("{layer}.t")),
                            ),
                            typography: true,
                            hoisted: self.placement.hoisted,
                            owner_reset: false,
                        },
                    })
                    .collect();
                Ok(Expansion::Typography {
                    preset: preset.name.clone(),
                    yielded,
                    members,
                })
            }
            EmissionInput::Keyframes { steps } => {
                let steps = steps
                    .iter()
                    .map(|(step, members)| {
                        let members = members
                            .iter()
                            .map(|member| Ok((member.property.clone(), member.effective_value()?)))
                            .collect::<Result<Vec<_>, ReplayError>>()?;
                        Ok((step.clone(), members))
                    })
                    .collect::<Result<Vec<_>, ReplayError>>()?;
                let record = RecordFootprint::Keyframes {
                    bucket: self.placement.bucket.clone(),
                    name: name.into(),
                    steps: steps.clone(),
                };
                Ok(Expansion::Keyframes { steps, record })
            }
        }
    }
}
