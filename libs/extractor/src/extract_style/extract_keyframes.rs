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

/// Hashes the original map/vector/seven-field stream without global declaration order.
struct KeyframeIdentity<'a>(&'a BTreeMap<String, Vec<ExtractStaticStyle>>);

impl Hash for KeyframeIdentity<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (key, styles) in self.0 {
            key.hash(state);
            styles.len().hash(state);
            for style in styles {
                style.property.hash(state);
                style.value.hash(state);
                style.level.hash(state);
                style.selector.hash(state);
                style.style_order.hash(state);
                style.layer.hash(state);
                style.theme_token_resolution.hash(state);
            }
        }
    }
}

impl ExtractStyleProperty for ExtractKeyframes {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        if css::atom_hoist::is_atom_hoist() {
            let mut content = String::new();
            for (step, styles) in &self.keyframes {
                content.push_str(&css::atom_name::hex(step));
                content.push('{');
                for style in styles {
                    content.push_str(&css::atom_name::hex(style.property()));
                    content.push(':');
                    content.push_str(&css::atom_name::hex(style.value()));
                    content.push(';');
                }
                content.push('}');
            }
            return StyleProperty::ClassName(keyframes_to_keyframes_name(&content, filename));
        }
        let mut hasher = DefaultHasher::new();
        KeyframeIdentity(&self.keyframes).hash(&mut hasher);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_style::extract_static_style::ThemeTokenResolution;
    use css::style_selector::StyleSelector;
    use rstest::rstest;
    use serial_test::serial;

    /// Main's pre-order layout, independently deriving the original hash contract.
    #[derive(Hash)]
    struct MainStaticStyle {
        property: String,
        value: String,
        level: u8,
        selector: Option<StyleSelector>,
        style_order: Option<u8>,
        layer: Option<String>,
        theme_token_resolution: ThemeTokenResolution,
    }

    fn main_keyframes() -> BTreeMap<String, Vec<MainStaticStyle>> {
        BTreeMap::from([
            (
                "from".to_string(),
                vec![
                    MainStaticStyle {
                        property: "opacity".to_string(),
                        value: "0".to_string(),
                        level: 0,
                        selector: None,
                        style_order: None,
                        layer: None,
                        theme_token_resolution: ThemeTokenResolution::CssVariable,
                    },
                    MainStaticStyle {
                        property: "transform".to_string(),
                        value: "rotate(0deg)".to_string(),
                        level: 2,
                        selector: Some(StyleSelector::from("hover")),
                        style_order: Some(1),
                        layer: Some("motion".to_string()),
                        theme_token_resolution: ThemeTokenResolution::FirstValue,
                    },
                ],
            ),
            ("to".to_string(), vec![]),
        ])
    }

    fn keyframes_with_order(order: u32) -> ExtractKeyframes {
        ExtractKeyframes {
            keyframes: main_keyframes()
                .into_iter()
                .map(|(key, styles)| {
                    let styles = styles
                        .into_iter()
                        .map(|style| ExtractStaticStyle {
                            property: style.property,
                            value: style.value,
                            level: style.level,
                            selector: style.selector,
                            style_order: style.style_order,
                            layer: style.layer,
                            theme_token_resolution: style.theme_token_resolution,
                            order,
                        })
                        .collect();
                    (key, styles)
                })
                .collect(),
        }
    }

    #[derive(Default)]
    struct HashWrites(Vec<Vec<u8>>);

    impl Hasher for HashWrites {
        fn finish(&self) -> u64 {
            0
        }

        fn write(&mut self, bytes: &[u8]) {
            self.0.push(bytes.to_vec());
        }
    }

    #[rstest]
    #[case(0)]
    #[case(47)]
    fn hash_stream_matches_main_when_order_changes(#[case] order: u32) {
        let mut expected = HashWrites::default();
        main_keyframes().hash(&mut expected);
        let keyframes = keyframes_with_order(order);
        let mut actual = HashWrites::default();

        KeyframeIdentity(&keyframes.keyframes).hash(&mut actual);

        assert_eq!(actual.0, expected.0);
    }

    #[rstest]
    #[case(None)]
    #[case(Some("keyframe-identity.tsx"))]
    #[serial]
    fn identifier_matches_main_when_only_order_changes(#[case] filename: Option<&str>) {
        let mut main_hasher = DefaultHasher::new();
        main_keyframes().hash(&mut main_hasher);
        let expected = keyframes_to_keyframes_name(&main_hasher.finish().to_string(), filename);

        let identifiers =
            [0, 47].map(
                |order| match keyframes_with_order(order).extract(filename) {
                    StyleProperty::ClassName(name) => name,
                    StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
                },
            );

        assert_eq!(identifiers, [expected.clone(), expected]);
    }

    #[rstest]
    #[case("color", "0")]
    #[case("opacity", "1")]
    #[serial]
    fn identifier_changes_when_property_or_value_changes(
        #[case] property: &str,
        #[case] value: &str,
    ) -> Result<(), &'static str> {
        let original = keyframes_with_order(0);
        let mut changed = original.clone();
        let style = &mut changed
            .keyframes
            .get_mut("from")
            .ok_or("missing fixture frame")?[0];
        style.property = property.to_string();
        style.value = value.to_string();
        let original_identifier = match original.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
        };

        let changed_identifier = match changed.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
        };

        assert_ne!(changed_identifier, original_identifier);
        Ok(())
    }
}
