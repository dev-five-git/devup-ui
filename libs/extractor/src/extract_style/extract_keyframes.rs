use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use css::keyframes_to_keyframes_name;

use crate::extract_style::{
    ExtractStyleProperty, extract_static_style::ExtractStaticStyle, style_property::StyleProperty,
};

#[derive(Debug, Default, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractKeyframes {
    pub keyframes: BTreeMap<String, Vec<ExtractStaticStyle>>,
}

impl ExtractStyleProperty for ExtractKeyframes {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let mut hasher = DefaultHasher::new();
        self.keyframes.hash(&mut hasher);
        // Format the u64 hash into a stack buffer instead of a throwaway heap
        // `String`; `keyframes_to_keyframes_name` only reads it as `&str` and
        // copies it into its own key, so the owned allocation was pure waste.
        let mut buf = [0u8; 20];
        let hash_key = write_u64(&mut buf, hasher.finish());
        StyleProperty::ClassName(keyframes_to_keyframes_name(hash_key, filename))
    }
}

/// Writes `value`'s decimal digits into the tail of `buf` and returns the
/// written slice as `&str`. A `u64` is at most 20 decimal digits, so `buf`
/// never overflows.
fn write_u64(buf: &mut [u8; 20], mut value: u64) -> &str {
    let mut pos = buf.len();
    loop {
        pos -= 1;
        buf[pos] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    // Only ASCII digits were written, so this slice is valid UTF-8.
    str::from_utf8(&buf[pos..]).unwrap_or("0")
}
