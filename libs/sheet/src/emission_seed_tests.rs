use crate::{
    StyleSheetProperty,
    counter_evidence::{Expansion, RecordFootprint, ReplayError},
    emission_seed::*,
    emission_seed_test_helpers::{declaration, preset, property, require_ok, seed},
};
use css::style_selector::StyleSelector;
use rstest::rstest;

#[rstest]
#[case("content", "\"$brand.name\"", "\"var(--brand-name)\"")]
#[case("--literal", "\"quoted\" $brand.name", "\"quoted\" var(--brand-name)")]
#[case("font-family", "'Roboto'", "Roboto")]
#[case("font-size", "14px", "14px")]
fn static_replays_effective_value_when_normalized_input_is_retained(
    #[case] property: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given
    let input = seed(EmissionInput::Static(declaration(property, value)));
    // When
    let Expansion::Static(records) = require_ok(input.replay("a")) else {
        panic!("static")
    };
    // Then
    let RecordFootprint::Property { record, .. } = &records[0] else {
        panic!("property")
    };
    assert_eq!(record.value, expected);
}

#[test]
fn first_value_replays_captured_24px_when_authored_token_is_independent() {
    // Given
    let mut declaration = declaration("font-size", "$size");
    declaration.resolution = Resolution::FirstValue;
    declaration.first_value = Some("24px".into());
    // When
    let result = seed(EmissionInput::Static(declaration)).replay("a");
    // Then
    assert_eq!(result, Ok(Expansion::Static(vec![property("24px")])));
}

#[test]
fn first_value_falls_back_when_lookup_was_absent() {
    // Given
    let mut declaration = declaration("font-size", "$size");
    declaration.resolution = Resolution::FirstValue;
    // When
    let result = seed(EmissionInput::Static(declaration)).replay("a");
    // Then
    assert_eq!(result, Ok(Expansion::Static(vec![property("var(--size)")])));
}

#[test]
fn typography_replays_sparse_frames_when_selected_at_base() {
    // Given
    let mut declaration = declaration("typography", "heading");
    declaration.preset = Some(preset());
    // When
    let Expansion::Typography { members, .. } =
        require_ok(seed(EmissionInput::Typography(declaration)).replay("a"))
    else {
        panic!("typography")
    };
    // Then
    let values: Vec<_> = members
        .iter()
        .map(|member| match member {
            RecordFootprint::Property { level, record, .. } => (
                *level,
                record.property.as_str(),
                record.value.as_str(),
                record.layer.as_deref(),
                record.typography,
            ),
            _ => panic!("property"),
        })
        .collect();
    assert_eq!(
        values,
        vec![
            (0, "font-size", "14px", Some("t"), true),
            (0, "font-weight", "400", Some("t"), true),
            (2, "font-family", "var(--fonts.heading)", Some("t"), true),
            (2, "font-size", "24px", Some("t"), true),
        ]
    );
}

#[test]
fn typography_merges_later_frame_when_start_is_above_frames() {
    // Given
    let mut declaration = declaration("typography", "heading|font-weight:1,garbage,font-size:bad");
    declaration.level = 3;
    declaration.layer = Some("authored".into());
    declaration.preset = Some(preset());
    // When
    let Expansion::Typography {
        members, yielded, ..
    } = require_ok(seed(EmissionInput::Typography(declaration)).replay("a"))
    else {
        panic!("typography")
    };
    // Then
    assert_eq!(
        yielded,
        vec![Yield {
            property: "font-weight".into(),
            from: 1
        }]
    );
    let values: Vec<_> = members
        .iter()
        .map(|member| match member {
            RecordFootprint::Property { level, record, .. } => (
                *level,
                record.property.as_str(),
                record.value.as_str(),
                record.layer.as_deref(),
            ),
            _ => panic!("property"),
        })
        .collect();
    assert_eq!(
        values,
        vec![
            (3, "font-size", "24px", Some("authored.t")),
            (3, "font-family", "var(--fonts.heading)", Some("authored.t"))
        ]
    );
}

#[test]
fn typography_keeps_lower_member_when_yield_starts_at_wider_level() {
    // Given
    let mut declaration = declaration("typography", "heading|font-size:2");
    declaration.preset = Some(preset());
    // When
    let Expansion::Typography { members, .. } =
        require_ok(seed(EmissionInput::Typography(declaration)).replay("a"))
    else {
        panic!("typography")
    };
    // Then
    let sizes: Vec<_> = members
        .iter()
        .filter_map(|member| match member {
            RecordFootprint::Property { level, record, .. } if record.property == "font-size" => {
                Some((*level, record.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sizes, vec![(0, "14px")]);
}

#[test]
fn typography_rejects_when_frozen_preset_name_disagrees() {
    // Given
    let mut declaration = declaration("typography", "other");
    declaration.preset = Some(preset());
    // When
    let result = seed(EmissionInput::Typography(declaration)).replay("a");
    // Then
    assert_eq!(result, Err(ReplayError::Preset));
}

#[test]
fn dynamic_replays_exact_consumer_and_plain_reset_when_important_and_conditional() {
    // Given
    let mut declaration = declaration("padding", "independent-expression");
    declaration.level = 2;
    declaration.selector = Some(StyleSelector::Selector("&:hover".into()));
    declaration.layer = Some("components".into());
    declaration.style_order = Some(3);
    let site = NumericSite {
        source: 7,
        at: 42,
        role: 2,
    };
    let mut input = seed(EmissionInput::Dynamic {
        declaration,
        variable: "---Sa-bg-c".into(),
        site: Some(site.clone()),
        important: true,
    });
    input.placement.hoisted = true;
    // When
    let result = input.replay("a");
    // Then
    assert_eq!(
        result,
        Ok(Expansion::Dynamic {
            variable: "---Sa-bg-c".into(),
            site: Some(site),
            important: true,
            consumer: Box::new(RecordFootprint::Property {
                bucket: "delivery.tsx".into(),
                order: 3,
                level: 2,
                record: StyleSheetProperty {
                    class_name: "a".into(),
                    property: "padding".into(),
                    value: "var(---Sa-bg-c) !important".into(),
                    selector: Some(StyleSelector::Selector("&:hover".into())),
                    layer: Some("components".into()),
                    typography: false,
                    hoisted: true,
                    owner_reset: false,
                }
            }),
            reset: Box::new(RecordFootprint::Property {
                bucket: "delivery.tsx".into(),
                order: 3,
                level: 0,
                record: StyleSheetProperty {
                    class_name: "a".into(),
                    property: "---Sa-bg-c".into(),
                    value: "initial".into(),
                    selector: None,
                    layer: None,
                    typography: false,
                    hoisted: true,
                    owner_reset: true,
                }
            }),
        })
    );
}
