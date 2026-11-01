use std::collections::BTreeMap;

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
        StyleProperty::ClassName(keyframes_to_keyframes_name(&content, filename))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_style::extract_static_style::ThemeTokenResolution;
    use css::style_selector::StyleSelector;
    use rstest::rstest;
    use serial_test::serial;

    #[test]
    #[serial]
    fn ordinary_keyframes_allocator_uses_lossless_content_identity() {
        // Given
        let _state = css::test_state::TestStateGuard::new();
        let mut keyframes = keyframes_with_order(47);
        keyframes
            .keyframes
            .get_mut("from")
            .unwrap_or_else(|| panic!("missing fixture frame"))
            .reverse();
        let content =
            "66726f6d{7472616e73666f726d:726f74617465283064656729;6f706163697479:30;}746f{}";
        // When
        let name = match keyframes.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("expected keyframes class name"),
        };
        // Then
        assert_eq!(name, "a");
        let map = css::class_map::get_class_map();
        assert_eq!(
            map[""].keys().cloned().collect::<Vec<_>>(),
            vec![format!("k-{content}")]
        );
    }

    fn keyframes_with_order(order: u32) -> ExtractKeyframes {
        ExtractKeyframes {
            keyframes: BTreeMap::from([
                (
                    "from".to_string(),
                    vec![
                        ExtractStaticStyle {
                            property: "opacity".to_string(),
                            value: "0".to_string(),
                            level: 0,
                            selector: None,
                            style_order: None,
                            layer: None,
                            theme_token_resolution: ThemeTokenResolution::CssVariable,
                            order,
                        },
                        ExtractStaticStyle {
                            property: "transform".to_string(),
                            value: "rotate(0deg)".to_string(),
                            level: 2,
                            selector: Some(StyleSelector::from("hover")),
                            style_order: Some(1),
                            layer: Some("motion".to_string()),
                            theme_token_resolution: ThemeTokenResolution::FirstValue,
                            order,
                        },
                    ],
                ),
                ("to".to_string(), vec![]),
            ]),
        }
    }

    #[rstest]
    #[case(None)]
    #[case(Some("keyframe-identity.tsx"))]
    #[serial]
    fn identifier_matches_when_only_authored_order_changes(#[case] filename: Option<&str>) {
        let _state = css::test_state::TestStateGuard::new();
        let identifiers =
            [0, 47].map(
                |order| match keyframes_with_order(order).extract(filename) {
                    StyleProperty::ClassName(name) => name,
                    StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
                },
            );

        assert_eq!(identifiers[0], identifiers[1]);
    }

    #[rstest]
    #[case(("opacity", "0"), ("color", "0"))]
    #[case(("opacity", "0"), ("opacity", "1"))]
    #[case(("a", "bc"), ("ab", "c"))]
    #[case(("a:b", "c;{}"), ("a", "b:c;{}"))]
    #[serial]
    fn identifier_changes_when_property_or_value_changes(
        #[case] before: (&str, &str),
        #[case] after: (&str, &str),
    ) -> Result<(), &'static str> {
        let _state = css::test_state::TestStateGuard::new();
        // Given
        let mut original = keyframes_with_order(0);
        let style = &mut original
            .keyframes
            .get_mut("from")
            .ok_or("missing fixture frame")?[0];
        style.property = before.0.into();
        style.value = before.1.into();
        let mut changed = original.clone();
        let style = &mut changed
            .keyframes
            .get_mut("from")
            .ok_or("missing fixture frame")?[0];
        style.property = after.0.into();
        style.value = after.1.into();
        let original_identifier = match original.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
        };

        // When
        let changed_identifier = match changed.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
        };

        // Then
        assert_ne!(changed_identifier, original_identifier);
        Ok(())
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    #[serial]
    fn identifier_changes_when_declarations_reorder_or_empty_step_is_removed(
        #[case] remove_empty_step: bool,
    ) -> Result<(), &'static str> {
        // Given
        let _state = css::test_state::TestStateGuard::new();
        let original = keyframes_with_order(0);
        let mut changed = original.clone();
        if remove_empty_step {
            changed.keyframes.remove("to");
        } else {
            changed
                .keyframes
                .get_mut("from")
                .ok_or("missing fixture frame")?
                .reverse();
        }
        // When
        let names = [original, changed].map(|frames| match frames.extract(None) {
            StyleProperty::ClassName(name) => name,
            StyleProperty::Variable { .. } => panic!("keyframes must extract a class name"),
        });
        // Then
        assert_ne!(names[0], names[1]);
        Ok(())
    }
}
