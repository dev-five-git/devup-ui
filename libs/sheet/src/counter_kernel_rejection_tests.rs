use super::{KernelError, authentic_support::*};
use crate::StyleSheet;

#[test]
#[serial_test::serial]
fn current_dynamic_is_rejected_when_ordinary_no_site_ir_reaches_retained_sheet() {
    // Given
    let _state = state();
    css::class_map::set_class_map(HashMap::from([("empty".into(), HashMap::new())]));
    let base = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    update(&mut sheet, &mut evidence, &base).required("retained base");
    sheet.cache_restore = crate::cache_snapshot::CacheRestore::Rejected;
    sheet.source_ids.insert("unrelated".into(), 17);
    let current = ExtractDynamicStyle::new("padding", 1, "spacing", None);
    assert_eq!(
        current.producer_policy(),
        extractor::extract_style::ProducerPolicy::Current
    );
    assert_eq!(current.site(), None);
    let items = styles([ExtractStyleValue::Dynamic(current)]);
    let before = capture(&sheet, &evidence);
    let mut called = false;
    // When
    let result = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
        attempt
            .prepare(
                &items,
                UpdateRequest {
                    raw_source: "current.tsx",
                    single_css: false,
                },
            )?
            .finish(|_, _| {
                called = true;
                Ok::<_, ()>(())
            })
    });
    // Then
    assert_eq!(
        result,
        Err(UpdateError::Kernel(KernelError::Producer(
            extractor::extract_style::CounterProducerError::WrongPolicy
        )))
    );
    assert!(!called);
    assert_eq!(capture(&sheet, &evidence), before);
}

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
    let preflight = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed_preflight = std::rc::Rc::clone(&preflight);
    let resolver = move |_: &str, _: &str| {
        let before = css::class_map::get_class_map();
        let candidate = parent
            .borrow()
            .as_ref()
            .map(|parent| parent.clone().at_role(0, 2));
        if let Some(candidate) = candidate {
            if candidate.site().is_some() {
                drop(parent.borrow_mut().take());
                *output.borrow_mut() = Some(candidate);
            } else {
                observed_preflight.set(true);
                assert_eq!(css::class_map::get_class_map(), before);
                assert!(output.borrow().is_none());
                assert!(parent.borrow().is_some());
            }
        }
        Some(extractor::ResolvedModule {
            path: "tone.ts".into(),
            code: "export const tone = 'red';".into(),
        })
    };
    extractor::extract_with_modules("unnumbered.tsx",
        "import { Box } from '@devup-ui/react'; import { tone } from './tone'; export const a = <Box color={tone} />;",
         extractor::ExtractOption::default(), false, &resolver).required("resolver");
    assert!(preflight.get());
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
