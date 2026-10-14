use super::counter_fixture_support::{fixture, state};
use extractor::extract_style::{
    CounterProducerError, ExtractDynamicStyle, ExtractKeyframes, ProducerPolicy,
    extract_static_style::ExtractStaticStyle,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[test]
#[serial_test::serial]
fn current_parents_reject_before_allocation_when_real_constructors_run_outside_fixture() {
    // Given: all ordinary public constructors remain Current with support enabled.
    let _state = state();
    let static_style = ExtractStaticStyle::new("color", "red", 0, None);
    let basic = ExtractStaticStyle::new_basic("color", "red", 0, None);
    let dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
    let frames = ExtractKeyframes::default();
    let before = HashMap::from([
        ("empty".into(), HashMap::new()),
        (String::new(), HashMap::from([("seed".into(), 9)])),
    ]);
    css::class_map::set_class_map(before.clone());
    // When: all genuine Current parents attempt the dormant adapter boundary.
    let errors = [
        static_style.counter_produce(None).err(),
        basic.counter_produce(Some("delivery")).err(),
        dynamic.counter_produce(None).err(),
        frames.counter_produce(None).err(),
    ];
    // Then: no allocator namespace or slot changes, including empty namespace presence.
    assert_eq!(errors, [Some(CounterProducerError::WrongPolicy); 4]);
    assert_eq!(css::class_map::get_class_map(), before);
}

#[test]
#[serial_test::serial]
fn current_child_rejects_before_allocation_when_parent_was_constructed_in_fixture() {
    // Given: a genuine Counter parent contains an ordinary Current child.
    let _state = state();
    let mut frames = fixture("parent", ExtractKeyframes::default);
    frames.keyframes.insert(
        "to".into(),
        vec![ExtractStaticStyle::new("opacity", "1", 0, None)],
    );
    let before = HashMap::from([("empty".into(), HashMap::new())]);
    css::class_map::set_class_map(before.clone());
    // When: the real keyframe adapter checks all retained construction policies.
    let result = frames.counter_produce(Some("delivery"));
    // Then: parent authenticity cannot authorize a Current child or reserve a hash slot.
    assert_eq!(result, Err(CounterProducerError::WrongPolicy));
    assert_eq!(css::class_map::get_class_map(), before);
}

#[test]
#[serial_test::serial]
fn unnumbered_site_rejects_when_public_at_role_runs_in_ordinary_resolver_scope() {
    // Given: a real Counter parent, without a public policy or site-field setter.
    let _state = state();
    let parent = fixture("parent", || {
        ExtractDynamicStyle::new("color", 0, "tone", None)
    });
    let retained = Rc::new(RefCell::new(Some(parent)));
    let captured = Rc::new(RefCell::new(None));
    let input = Rc::clone(&retained);
    let output = Rc::clone(&captured);
    let resolver = move |_: &str, _: &str| {
        if let Some(parent) = input.borrow_mut().take() {
            *output.borrow_mut() = Some(parent.at_role(0, 2));
        }
        Some(extractor::ResolvedModule {
            path: "tone.ts".into(),
            code: "export const tone = 'red';".into(),
        })
    };
    let source = "import { Box } from '@devup-ui/react'; import { tone } from './tone'; export const a = <Box color={tone} />;";
    let ordinary = extractor::extract_with_modules(
        "unnumbered.tsx",
        source,
        extractor::ExtractOption::default(),
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("ordinary resolver fixture: {error}"));
    assert_ne!(ordinary.code, "");
    let style = captured
        .borrow_mut()
        .take()
        .unwrap_or_else(|| panic!("resolver captured parent"));
    let expected = css::Site {
        file: css::sparse_site::SourceFile::from_source("unnumbered.tsx", source),
        at: 0,
        role: 2,
    };
    assert_eq!(style.site(), Some(&expected));
    assert!(matches!(
        expected.file,
        css::sparse_site::SourceFile::Unnumbered(_)
    ));
    assert_eq!(style.producer_policy(), ProducerPolicy::CounterOriginal(0));
    let before = css::class_map::get_class_map();
    // When: authentic Counter policy is paired with a publicly attached unnumbered site.
    let result = style.counter_produce(None);
    // Then: typed rejection occurs before any allocator write or visible hash fallback.
    assert_eq!(result, Err(CounterProducerError::UnnumberedSite));
    assert_eq!(css::class_map::get_class_map(), before);
}
