//! Versioned semantic descriptors shared by extraction and sheet registration.

use crate::{
    Naming,
    naming::escape_into,
    style_selector::{AtRuleKind, StyleSelector},
};

/// The declaration identity, without runtime values or delivery/cleanup owners.
pub struct AtomContent<'a> {
    pub property: &'a str,
    pub value: Option<&'a str>,
    pub naming: Naming,
    pub level: u8,
    pub order: u8,
    pub selector: Option<&'a StyleSelector>,
    pub layer: Option<&'a str>,
    pub dynamic: bool,
}

/// Exact versioned bytes plus their injective, escaped text representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentName {
    pub descriptor: Vec<u8>,
    pub lossless: String,
    pub domain: char,
}

fn count(bytes: &mut Vec<u8>, length: usize) {
    bytes.extend_from_slice(&u64::try_from(length).unwrap_or(u64::MAX).to_be_bytes());
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    count(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn option(bytes: &mut Vec<u8>, value: Option<&str>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        text(bytes, value);
    }
}

const fn kind(kind: AtRuleKind) -> u8 {
    match kind {
        AtRuleKind::Media => 0,
        AtRuleKind::Supports => 1,
        AtRuleKind::Container => 2,
        AtRuleKind::Layer => 3,
    }
}

fn selector(bytes: &mut Vec<u8>, lossless: &mut String, value: Option<&StyleSelector>) {
    match value {
        None => bytes.push(0),
        Some(StyleSelector::Selector(value)) => {
            bytes.push(1);
            text(bytes, value);
            lossless.push_str("-s");
            escape_into(lossless, value);
        }
        Some(StyleSelector::Global(value, _)) => {
            bytes.push(2);
            text(bytes, value);
            lossless.push_str("-g");
            escape_into(lossless, value);
        }
        Some(StyleSelector::At {
            kind: rule_kind,
            query,
            selector: inner,
            outer,
            ..
        }) => {
            bytes.push(3);
            count(bytes, outer.len());
            lossless.push_str("-a");
            for rule in outer {
                bytes.push(kind(rule.kind));
                text(bytes, &rule.query);
                lossless.push(char::from(b'0' + kind(rule.kind)));
                escape_into(lossless, &rule.query);
                lossless.push('-');
            }
            bytes.push(kind(*rule_kind));
            text(bytes, query);
            option(bytes, inner.as_deref());
            lossless.push_str("e-");
            lossless.push(char::from(b'0' + kind(*rule_kind)));
            escape_into(lossless, query);
            if let Some(inner) = inner {
                lossless.push_str("-s");
                escape_into(lossless, inner);
            }
        }
    }
}

impl AtomContent<'_> {
    /// Fixed order: version, variant, provenance, property, value, level, order,
    /// structured selector/at-rules, layer. Strings/counts use big-endian u64.
    #[must_use]
    pub fn content(&self) -> ContentName {
        let property = self.property.trim();
        let value = self.value.map(|value| {
            let normalized = crate::optimize_value::optimize_value(value);
            crate::content_value::emitted(&normalized).into_owned()
        });
        let mut bytes = vec![
            1,
            if self.dynamic { 2 } else { 1 },
            match self.naming {
                Naming::Own => 0,
                Naming::Risky => 1,
            },
        ];
        text(&mut bytes, property);
        option(&mut bytes, value.as_deref());
        bytes.extend_from_slice(&[self.level, self.order]);
        let mut lossless = String::new();
        if self.dynamic {
            lossless.push_str("d-");
        }
        escape_into(&mut lossless, property);
        if let Some(value) = value.as_deref() {
            lossless.push_str("-v");
            escape_into(&mut lossless, value);
        }
        if self.level != 0 {
            lossless.push_str("-l");
            crate::write_u8(&mut lossless, self.level);
        }
        if self.order != 255 {
            lossless.push_str("-o");
            crate::write_u8(&mut lossless, self.order);
        }
        selector(&mut bytes, &mut lossless, self.selector);
        option(&mut bytes, self.layer);
        if let Some(layer) = self.layer {
            lossless.push_str("-y");
            escape_into(&mut lossless, layer);
        }
        ContentName {
            descriptor: bytes,
            lossless,
            domain: match self.naming {
                Naming::Own => 'O',
                Naming::Risky => 'R',
            },
        }
    }
}

impl ContentName {
    /// Exact normalized source bytes, in the unnumbered site's separate domain.
    #[must_use]
    pub fn source(source: &str) -> Self {
        let mut lossless = String::new();
        escape_into(&mut lossless, source);
        Self {
            descriptor: source.as_bytes().to_vec(),
            lossless,
            domain: 'U',
        }
    }

    /// Keyframe step and declaration order are significant, exactly as emitted.
    #[must_use]
    pub fn keyframes(steps: &[(String, Vec<(String, String)>)]) -> Self {
        let mut bytes = vec![1, 3];
        let mut lossless = String::new();
        count(&mut bytes, steps.len());
        for (step, properties) in steps {
            text(&mut bytes, step);
            count(&mut bytes, properties.len());
            lossless.push('s');
            escape_into(&mut lossless, step);
            for (property, value) in properties {
                text(&mut bytes, property);
                text(&mut bytes, value);
                lossless.push_str("-p");
                escape_into(&mut lossless, property);
                lossless.push_str("-v");
                escape_into(&mut lossless, value);
            }
            lossless.push_str("-e-");
        }
        Self {
            descriptor: bytes,
            lossless,
            domain: 'K',
        }
    }

    /// Lossless wins ties; the prefix and domain are included in both lengths.
    #[must_use]
    pub fn name(&self, prefix: &str) -> String {
        self.name_with_bits(prefix, crate::content_hash::FingerprintBits::PRODUCTION)
    }

    /// Width is an input to naming, never global state or witness-dependent.
    #[must_use]
    pub fn name_with_bits(
        &self,
        prefix: &str,
        bits: crate::content_hash::FingerprintBits,
    ) -> String {
        if self.lossless.len() <= bits.digits() {
            format!("{prefix}{}L{}", self.domain, self.lossless)
        } else {
            format!(
                "{prefix}{}H{}",
                self.domain,
                crate::content_hash::fingerprint_with_bits(&self.descriptor, bits)
            )
        }
    }
}
