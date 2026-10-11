use super::authentic_support::*;
use crate::{StyleSheet, theme::Typographies};

#[test]
#[serial_test::serial]
fn callback_renders_prospective_live_sheet_when_real_update_is_prepared() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    sheet.theme.breakpoints = vec![0, 600];
    sheet.source_ids.insert("untouched".into(), 19);
    sheet.cache_restore = crate::cache_snapshot::CacheRestore::Rejected;
    // When
    let rendered = CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            fixture("a", || {
                let items = styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
                    "color", "red", 1, None,
                ))]);
                attempt
                    .prepare(
                        &items,
                        UpdateRequest {
                            raw_source: "a",
                            single_css: false,
                        },
                    )?
                    .finish(|sheet, effects| {
                        assert_eq!(sheet.theme.breakpoints, vec![0, 600]);
                        assert_eq!(
                            effects,
                            UpdateEffects {
                                collected: true,
                                updated_base_style: false,
                                default_collected: false
                            }
                        );
                        Ok::<_, ()>(sheet.create_css(Some("a"), false))
                    })
            })
        })
        .required("render");
    // Then
    assert!(rendered.contains("600px"));
    assert!(rendered.contains("color:red"));
    assert_eq!(sheet.source_ids, BTreeMap::from([("untouched".into(), 19)]));
    assert!(matches!(
        sheet.cache_restore,
        crate::cache_snapshot::CacheRestore::Rejected
    ));
}

#[test]
#[serial_test::serial]
fn insertion_effects_are_false_when_empty_or_identical_non_global_update_runs() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    update(&mut sheet, &items).required("initial");
    // When
    let duplicate = update(&mut sheet, &items).required("duplicate");
    // Then
    assert_eq!(
        duplicate,
        UpdateEffects {
            collected: false,
            updated_base_style: false,
            default_collected: false
        }
    );
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("retained")
            .candidates()
            .count(),
        1
    );
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn literal_emission_has_real_flags_when_atom_mode_changes_import_and_font_signaling(
    #[case] atom: bool,
) {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(atom.then_some(3));
    let mut sheet = StyleSheet::default();
    let output = extractor::extract_without_source_map("owner.tsx",
        "import { globalCss } from '@devup-ui/react'; globalCss({imports:['https://example.test/a.css'],fontFaces:[{fontFamily:'Example'}]});",
        extractor::ExtractOption::default()).required("literal extraction");
    let mut items = output.styles;
    items.insert(ExtractStyleValue::Typography("standalone".into()));
    // When
    let effects = update(&mut sheet, &items).required("literals");
    // Then
    assert_eq!(
        effects,
        UpdateEffects {
            collected: false,
            updated_base_style: atom,
            default_collected: false
        }
    );
    assert_eq!(sheet.global_css_files, BTreeSet::from(["owner.tsx".into()]));
    assert_eq!(
        sheet.imports["owner.tsx"],
        BTreeSet::from(["https://example.test/a.css".into()])
    );
    assert_eq!(sheet.font_faces["owner.tsx"].len(), 1);
    assert_eq!(css::class_map::get_class_map().len(), 0);
}

#[test]
#[serial_test::serial]
fn raw_css_is_base_only_when_literal_ir_reaches_real_kernel() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let items = styles(extractor::extract_without_source_map("a.tsx",
        "import * as stylex from '@stylexjs/stylex'; const vars = stylex.defineVars({tone:'red'});",
        extractor::ExtractOption::default()).required("literal extraction").styles);
    // When
    let effects = update(&mut sheet, &items).required("css");
    // Then
    assert_eq!(
        effects,
        UpdateEffects {
            collected: false,
            updated_base_style: true,
            default_collected: false
        }
    );
    assert_eq!(sheet.css["a.tsx"].len(), 1);
    let css = &sheet.css["a.tsx"].iter().next().required("css").css;
    assert!(css.starts_with(":root{"));
    assert!(css.contains(":red"));
}

#[test]
#[serial_test::serial]
fn sparse_preset_yields_are_frozen_when_real_typography_and_keyframe_children_are_used() {
    // Given
    let _state = state();
    let _presets = Presets::save();
    let mut sheet = StyleSheet::default();
    sheet.theme.typography.insert(
        "heading".into(),
        Typographies(vec![
            Some(frame("16px", Some("700"))),
            None,
            Some(frame("24px", None)),
        ]),
    );
    css::content_typography::set(BTreeMap::from([(
        "heading".into(),
        sheet
            .theme
            .typography_declarations("heading", 0)
            .into_iter()
            .map(|(level, property, value)| (level, property.into(), value))
            .collect(),
    )]));
    let items = fixture("a", || {
        let typography = ExtractStaticStyle::new("typography", "heading|font-size:2", 0, None);
        let mut frames = ExtractKeyframes::default();
        frames
            .keyframes
            .insert("from".into(), vec![typography.clone(), typography.clone()]);
        frames.keyframes.insert("to".into(), vec![]);
        styles([
            ExtractStyleValue::Static(typography),
            ExtractStyleValue::Keyframes(frames),
        ])
    });
    // When
    update(&mut sheet, &items).required("typography");
    // Then
    assert_eq!(sheet.properties["a"][&255][&0].len(), 2);
    assert_eq!(sheet.properties["a"][&255].len(), 1);
    let frames = sheet.keyframes["a"].values().next().required("frames");
    assert_eq!(frames["from"].len(), 2);
    assert_eq!(frames["from"][0], frames["from"][1]);
    assert_eq!(frames["to"], Vec::<(String, String)>::new());
    assert_eq!(css::class_map::get_class_map()["D9-0"].len(), 2);
}

#[test]
#[serial_test::serial]
fn missing_preset_and_first_value_are_empty_or_raw_when_genuine_lookup_is_missing() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let items = fixture("a", || {
        styles([
        ExtractStyleValue::Static(ExtractStaticStyle::new("typography", "kernel-missing", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new("content", "'literal'", 0, None)
            .with_theme_token_resolution(extractor::extract_style::extract_static_style::ThemeTokenResolution::FirstValue)),
    ])
    });
    // When
    update(&mut sheet, &items).required("missing lookups");
    // Then
    assert_eq!(sheet.properties["a"][&255][&0].len(), 1);
    assert_eq!(
        sheet.properties["a"][&255][&0]
            .iter()
            .next()
            .required("content")
            .value,
        "'literal'"
    );
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("owned state")
            .candidates()
            .count(),
        2
    );
}
