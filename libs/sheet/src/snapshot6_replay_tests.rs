use super::test_support::*;
use extractor::extract_style::extract_static_style::ThemeTokenResolution;

struct Values;
impl Drop for Values {
    fn drop(&mut self) {
        css::theme_tokens::set_theme_token_values(BTreeMap::new(), BTreeMap::new());
    }
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn frozen_first_value_replays_when_actual_token_lookup_changes(#[case] present: bool) {
    // Given
    let _guard = state();
    let _values = Values;
    if present {
        css::theme_tokens::set_theme_token_values(
            BTreeMap::from([("token".into(), "16px".into())]),
            BTreeMap::new(),
        );
    }
    let items = fixture("a", || {
        let member = ExtractStaticStyle::new("width", "$token", 0, None)
            .with_theme_token_resolution(ThemeTokenResolution::FirstValue);
        let mut frames = ExtractKeyframes::default();
        frames.keyframes.insert("from".into(), vec![member.clone()]);
        styles([
            ExtractStyleValue::Static(member),
            ExtractStyleValue::Keyframes(frames),
        ])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("update");
    let bytes = encoded(&mut sheet);
    css::theme_tokens::set_theme_token_values(
        BTreeMap::from([("token".into(), "99px".into())]),
        BTreeMap::new(),
    );
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    let expected = if present { "16px" } else { "var(--token)" };
    assert_eq!(
        target.properties["a"][&255][&0]
            .iter()
            .next()
            .required("property")
            .value,
        expected
    );
    assert_eq!(
        target.keyframes["a"].values().next().required("keyframes")["from"],
        vec![("width".into(), expected.into())]
    );
}

#[rstest::rstest]
#[case(3)]
#[case(5)]
#[serial_test::serial]
fn frozen_presets_adopt_when_fresh_theme_and_registry_differ(#[case] family: u8) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(family);
    let bytes = encoded(&mut original);
    let expected = super::super::records::capture(&original)
        .into_iter()
        .collect::<rustc_hash::FxHashSet<_>>();
    let mut target = StyleSheet::default();
    target.set_theme(crate::theme::Theme {
        typography: BTreeMap::from([(
            "heading".into(),
            crate::theme::Typographies(vec![Some(frame("99px", Some("900")))]),
        )]),
        ..Default::default()
    });
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(
        super::super::records::capture(&target)
            .into_iter()
            .collect::<rustc_hash::FxHashSet<_>>(),
        expected
    );
    assert_eq!(
        target.theme.typography["heading"].0[0]
            .as_ref()
            .required("fresh frame")
            .font_size
            .as_deref(),
        Some("99px")
    );
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn adopted_state_supports_real_kernel_work_when_identical_cold_inputs_are_compiled(
    #[case] family: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(family);
    let bytes = encoded(&mut original);
    fresh();
    let mut target = StyleSheet::default();
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    let items = fixture("b", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "opacity", "1", 0, None,
        ))])
    });
    // When
    let effects = update(&mut target, &items).required("prospective kernel");
    // Then
    assert!(effects.collected);
    assert!(
        target.properties["a"][&255][&0]
            .iter()
            .any(|record| record.property == "opacity" && record.value == "1")
    );
    assert_eq!(css::file_map::original_id("b"), Some(1));
}
