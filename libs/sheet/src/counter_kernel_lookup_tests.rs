use super::authentic_support::*;
use crate::StyleSheet;
use extractor::extract_style::extract_static_style::ThemeTokenResolution;

struct Values;
impl Drop for Values {
    fn drop(&mut self) {
        css::theme_tokens::set_theme_token_values(BTreeMap::new(), BTreeMap::new());
    }
}

#[test]
#[serial_test::serial]
fn first_value_is_frozen_for_static_and_keyframes_when_registry_later_changes() {
    // Given
    let _state = state();
    let _values = Values;
    css::theme_tokens::set_theme_token_values(
        BTreeMap::from([("kernelToken".into(), "16px".into())]),
        BTreeMap::new(),
    );
    let items = fixture("a", || {
        let member = ExtractStaticStyle::new("width", "$kernelToken", 0, None)
            .with_theme_token_resolution(ThemeTokenResolution::FirstValue);
        let mut frames = ExtractKeyframes::default();
        frames.keyframes.insert("from".into(), vec![member.clone()]);
        styles([
            ExtractStyleValue::Static(member),
            ExtractStyleValue::Keyframes(frames),
        ])
    });
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    update(&mut sheet, &mut evidence, &items).required("initial");
    css::theme_tokens::set_theme_token_values(
        BTreeMap::from([("kernelToken".into(), "24px".into())]),
        BTreeMap::new(),
    );
    // When
    update(&mut sheet, &mut evidence, &styles([])).required("historical frozen replay");
    // Then
    assert_eq!(
        sheet.properties["a"][&255][&0]
            .iter()
            .next()
            .required("property")
            .value,
        "16px"
    );
    assert_eq!(
        sheet.keyframes["a"].values().next().required("keyframes")["from"],
        vec![("width".into(), "16px".into())]
    );
    assert_eq!(css::class_map::get_class_map()["D9-0"].len(), 2);
}

#[test]
#[serial_test::serial]
fn plan_and_sheet_restore_when_first_atom_freeze_is_followed_by_output_error() {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(usize::MAX));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let before = capture(&sheet, &evidence);
    // When
    let result = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
        attempt
            .prepare(
                &styles([]),
                UpdateRequest {
                    raw_source: "a",
                    single_css: false,
                },
            )?
            .finish(|sheet, _| {
                assert_eq!(sheet.atom_plan, Some(BTreeSet::new()));
                Err::<(), _>(())
            })
    });
    // Then
    assert_eq!(result, Err(UpdateError::Output(())));
    assert_eq!(capture(&sheet, &evidence), before);
}

#[test]
#[serial_test::serial]
fn outer_typed_owner_restores_inner_success_when_enclosing_exact_scope_aborts() {
    // Given
    let _state = state();
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let before = capture(&sheet, &evidence);
    // When
    let result = css::admission::with_admission(|| {
        let publication = super::publication::Publication::new(&mut sheet, &mut evidence);
        css::exact_attempt::with_exclusive_attempt(|| {
            let items = fixture("a", || {
                styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                    "color", 0, "tone", None,
                ))])
            });
            update(publication.sheet, publication.evidence, &items).required("inner success");
            Err::<(), _>("outer abort")
        })
    });
    // Then
    assert_eq!(result, Err("outer abort"));
    assert_eq!(capture(&sheet, &evidence), before);
}
