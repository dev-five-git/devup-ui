use super::{KernelError, authentic_support::*};
use crate::StyleSheet;

#[test]
#[serial_test::serial]
fn current_child_is_rejected_when_genuine_keyframe_parent_contains_current_member() {
    // Given
    let _state = state();
    let member = ExtractStaticStyle::new("opacity", "0", 0, None);
    let items = fixture("a", || {
        let mut frames = ExtractKeyframes::default();
        frames.keyframes.insert("from".into(), vec![member]);
        styles([ExtractStyleValue::Keyframes(frames)])
    });
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let before = capture(&sheet, &evidence);
    // When
    let result = update(&mut sheet, &mut evidence, &items);
    // Then
    assert_eq!(
        result,
        Err(UpdateError::Kernel(KernelError::Producer(
            extractor::extract_style::CounterProducerError::WrongPolicy
        )))
    );
    assert_eq!(capture(&sheet, &evidence), before);
}

#[test]
#[serial_test::serial]
fn unnumbered_site_is_rejected_when_genuine_parent_is_attached_outside_fixture() {
    // Given
    let _state = state();
    let parent = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
    let parent = std::rc::Rc::new(std::cell::RefCell::new(Some(parent)));
    let captured = std::rc::Rc::new(std::cell::RefCell::new(None));
    let output = std::rc::Rc::clone(&captured);
    let resolver = move |_: &str, _: &str| {
        if let Some(parent) = parent.borrow_mut().take() {
            *output.borrow_mut() = Some(parent.at_role(0, 2));
        }
        Some(extractor::ResolvedModule {
            path: "tone.ts".into(),
            code: "export const tone = 'red';".into(),
        })
    };
    extractor::extract_with_modules("unnumbered.tsx",
        "import { Box } from '@devup-ui/react'; import { tone } from './tone'; export const a = <Box color={tone} />;",
        extractor::ExtractOption::default(), false, &resolver).required("resolver");
    let items = styles([ExtractStyleValue::Dynamic(
        captured.borrow_mut().take().required("captured"),
    )]);
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let before = capture(&sheet, &evidence);
    // When
    let result = update(&mut sheet, &mut evidence, &items);
    // Then
    assert_eq!(
        result,
        Err(UpdateError::Kernel(KernelError::Producer(
            extractor::extract_style::CounterProducerError::UnnumberedSite
        )))
    );
    assert_eq!(capture(&sheet, &evidence), before);
}

#[test]
#[serial_test::serial]
fn raw_preset_mismatch_is_rejected_when_live_theme_and_registry_disagree() {
    // Given
    let _state = state();
    let _presets = Presets::save();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "typography",
            "heading",
            0,
            None,
        ))])
    });
    css::content_typography::set(BTreeMap::from([(
        "heading".into(),
        vec![(0, "font-size".into(), "16px".into())],
    )]));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let before = capture(&sheet, &evidence);
    // When
    let result = update(&mut sheet, &mut evidence, &items);
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Preset)));
    assert_eq!(capture(&sheet, &evidence), before);
}
