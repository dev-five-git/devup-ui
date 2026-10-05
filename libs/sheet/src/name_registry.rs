use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use css::{
    content_hash::FingerprintBits,
    style_origin::{RealLocation, StyleOrigin},
};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameClaim {
    pub descriptor: Vec<u8>,
    pub content: String,
    pub origin: Option<StyleOrigin>,
    pub location: RealLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameError {
    pub name: String,
    pub first: Box<NameClaim>,
    pub second: Box<NameClaim>,
}

impl Display for NameError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for claim in [&self.first, &self.second] {
            writeln!(
                f,
                "{}: generated style name `{}` cannot be used at build time: also names a different declaration; complete content: `{}`; descriptor: {:02x?}",
                claim.location, self.name, claim.content, claim.descriptor
            )?;
        }
        write!(
            f,
            "change one declaration, or increase the fixed fingerprint width and rebuild all caches"
        )
    }
}
impl std::error::Error for NameError {}

pub type NameRegistry = BTreeMap<String, NameClaim>;

/// Validate the whole batch first; an error leaves both registries untouched.
pub fn preflight(
    existing: &NameRegistry,
    incoming: impl IntoIterator<Item = (String, NameClaim)>,
) -> Result<NameRegistry, NameError> {
    let mut batch = NameRegistry::new();
    for (name, mut claim) in incoming {
        for previous in [existing.get(&name), batch.get(&name)]
            .into_iter()
            .flatten()
        {
            if previous.descriptor != claim.descriptor {
                let (first, second) = if (&previous.location, &previous.descriptor)
                    <= (&claim.location, &claim.descriptor)
                {
                    (previous.clone(), claim)
                } else {
                    (claim, previous.clone())
                };
                return Err(NameError {
                    name,
                    first: Box::new(first),
                    second: Box::new(second),
                });
            }
            if previous.location < claim.location {
                claim.location.clone_from(&previous.location);
                claim.origin.clone_from(&previous.origin);
            }
        }
        batch.insert(name, claim);
    }
    Ok(batch)
}

pub(crate) fn claim(
    style: &ExtractStyleValue,
    source: (&str, Option<&str>),
    bits: FingerprintBits,
) -> Option<(String, NameClaim)> {
    let (content, origin) = match style {
        ExtractStyleValue::Static(style) => {
            if css::naming::private_counter(
                source.1,
                style.naming,
                style.style_order.unwrap_or(255),
            )
            .is_some()
            {
                return None;
            }
            (style.content_name(), &style.origin)
        }
        ExtractStyleValue::Dynamic(style) => {
            if css::naming::private_counter(
                source.1,
                style.naming(),
                style.style_order().unwrap_or(255),
            )
            .is_some()
            {
                return None;
            }
            (style.content_name(), &style.origin)
        }
        ExtractStyleValue::Keyframes(style) => (style.content_name(), &style.origin),
        ExtractStyleValue::Typography(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_) => return None,
    };
    let name = content.name_with_bits(css::get_prefix().as_deref().unwrap_or_default(), bits);
    let location = origin
        .location()
        .unwrap_or_else(|| RealLocation::ModuleExport {
            file: source.0.into(),
            binding: None,
        });
    let description = format!("{style:?}; lossless={}", content.lossless);
    Some((
        name,
        NameClaim {
            descriptor: content.descriptor,
            content: description,
            origin: origin.0.as_deref().cloned(),
            location,
        },
    ))
}
