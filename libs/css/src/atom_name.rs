use crate::{atom_hoist::is_hoisted_bucket, file_map::canonical, style_selector::StyleSelector};
use std::fmt::Write;

/// Lossless byte encoding with no delimiter or CSS identifier ambiguity.
pub fn hex(value: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        result.push(char::from(DIGITS[usize::from(byte >> 4)]));
        result.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    result
}

/// Names identify placement, never the order a source was encountered.
pub fn scope(filename: Option<&str>) -> String {
    match filename {
        None => "g".to_string(),
        Some(file) if is_hoisted_bucket(file) => "h".to_string(),
        Some(file) => format!("l-{}", hex(&canonical(file))),
    }
}

/// Structural selector identity excludes the source owner used only for cleanup.
pub fn selector_key(selector: Option<&StyleSelector>, layer: Option<&str>) -> String {
    let mut key = match selector {
        None => "n".to_string(),
        Some(StyleSelector::Selector(text)) => format!("s-{}", hex(text)),
        Some(StyleSelector::Global(text, _)) => format!("g-{}", hex(text)),
        Some(StyleSelector::At {
            kind,
            query,
            selector,
            outer,
            file: _,
        }) => {
            let mut key = String::from("a");
            for rule in outer {
                let _ = write!(key, "-{}-{}", rule.kind, hex(&rule.query));
            }
            let _ = write!(key, "-end-{kind}-{}-", hex(query));
            match selector {
                None => key.push('n'),
                Some(text) => {
                    let _ = write!(key, "s-{}", hex(text));
                }
            }
            key
        }
    };
    match layer {
        None => key.push_str("-n"),
        Some(layer) => {
            let _ = write!(key, "-s-{}", hex(layer));
        }
    }
    key
}
