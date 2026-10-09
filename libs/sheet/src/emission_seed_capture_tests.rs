use crate::emission_seed::*;
use crate::emission_seed_test_helpers::require_ok;

#[test]
fn capture_retains_normalized_value_when_supplied_first_value_is_24px() {
    // Given
    use extractor::extract_style::extract_static_style::{
        ExtractStaticStyle, ThemeTokenResolution,
    };
    let style = ExtractStaticStyle::new("width", "$size", 2, None)
        .with_theme_token_resolution(ThemeTokenResolution::FirstValue);
    // When
    let captured = DeclarationSeed::capture(&style, Some("24px".into()), None);
    // Then
    assert_eq!(
        (
            captured.value.as_str(),
            captured.level,
            require_ok(captured.effective_value())
        ),
        ("$size", 2, "24px".into())
    );
}

#[test]
fn capture_keeps_css_variable_policy_when_supplied_lookup_is_irrelevant() {
    // Given
    use extractor::extract_style::extract_static_style::ExtractStaticStyle;
    let style = ExtractStaticStyle::new("width", "$size", 0, None);
    // When
    let captured = DeclarationSeed::capture(&style, Some("24px".into()), None);
    // Then
    assert_eq!(captured.effective_value(), Ok("var(--size)".into()));
}

#[test]
fn frame_capture_replays_raw_fields_when_quoted_family_and_token_size_are_used() {
    // Given
    let frame = crate::theme::Typography {
        font_family: Some(" 'Open Sans' ".into()),
        font_size: Some("$size".into()),
        font_style: Some("italic".into()),
        font_weight: Some("700".into()),
        line_height: Some("1.5".into()),
        letter_spacing: Some("1px".into()),
        text_transform: Some("uppercase".into()),
    };
    // When
    let preset = FrozenPreset {
        name: "frozen".into(),
        frames: vec![Some(FrozenTypography::from(&frame))],
    };
    let declarations = preset.declarations(0, &[]);
    // Then
    assert_eq!(
        declarations,
        vec![
            (0, "font-family".into(), "'Open Sans'".into()),
            (0, "font-size".into(), "var(--size)".into()),
            (0, "font-style".into(), "italic".into()),
            (0, "font-weight".into(), "700".into()),
            (0, "line-height".into(), "1.5".into()),
            (0, "letter-spacing".into(), "1px".into()),
            (0, "text-transform".into(), "uppercase".into()),
        ]
    );
}
