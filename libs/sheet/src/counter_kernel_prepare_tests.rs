use super::{KernelError, authentic_support::*};
use crate::StyleSheet;

#[test]
#[serial_test::serial]
fn retained_slot_is_used_when_genuine_constructor_reaches_live_kernel() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let maps = HashMap::from([
        ("empty".into(), HashMap::new()),
        (
            "D9-0".into(),
            HashMap::from([("color-0-red--255-a".into(), 7)]),
        ),
    ]);
    css::class_map::set_class_map(maps.clone());
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    // When
    let effects = update(&mut sheet, &mut evidence, &items).required("live update");
    // Then
    assert!(effects.collected);
    assert_eq!(css::class_map::get_class_map(), maps);
    assert_eq!(
        sheet.properties["a"][&255][&0]
            .iter()
            .next()
            .required("property")
            .class_name,
        "pa-h"
    );
    assert_eq!(css::file_map::get_file_map().get_by_left("a"), Some(&0));
}

#[test]
#[serial_test::serial]
fn shared_class_precedes_variable_when_real_traversal_interleaves_static_dynamic_keyframes() {
    // Given
    let _state = state();
    css::class_map::set_class_map(HashMap::from([(
        String::new(),
        HashMap::from([("seed".into(), 9)]),
    )]));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    // When
    let result = CounterSheet::new(&mut sheet, &mut evidence).with_attempt(|attempt| {
        fixture("a", || {
            let items = styles([
                ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
                ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 1, "tone", None)),
                ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
            ]);
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: true,
                    },
                )?
                .finish(|_, effects| Ok::<_, ()>(effects))
        })
    });
    // Then
    assert!(result.required("update").collected);
    let maps = css::class_map::get_class_map();
    assert_eq!(maps[""]["seed"], 9);
    assert_eq!(maps[""]["color-0-red--255"], 1);
    assert_eq!(maps[""]["color-1---255"], 2);
    assert_eq!(maps[""]["color-1-"], 3);
    assert_eq!(maps[""].len(), 5);
    assert_eq!(
        sheet.properties[""][&255][&0]
            .iter()
            .find(|record| record.owner_reset)
            .required("reset")
            .property,
        "--pd"
    );
    assert_eq!(
        evidence
            .retained
            .as_ref()
            .required("sidecar")
            .candidates
            .len(),
        3
    );
}

#[test]
#[serial_test::serial]
fn parent_site_bindings_are_separate_when_assignment_is_attached_in_another_fixture() {
    // Given
    let _state = state();
    let parent = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
    let child = fixture("b", || parent.at_role(2, 1));
    let items = styles([ExtractStyleValue::Dynamic(child)]);
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    // When
    update(&mut sheet, &mut evidence, &items).required("update");
    // Then
    assert_eq!(
        css::class_map::get_class_map(),
        HashMap::from([(
            "D9-0".into(),
            HashMap::from([("color-0-var(---pSb-c-b)--255-a".into(), 0)])
        )])
    );
    let retained = evidence.retained.as_ref().required("sidecar");
    assert_eq!(
        retained.authority.originals,
        BTreeMap::from([("a".into(), 0), ("b".into(), 1)])
    );
    assert_eq!(sheet.properties["a"][&255][&0].len(), 2);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn baseline_modes_preserve_maps_when_debug_or_atom_kernel_runs(#[case] atom: bool) {
    // Given
    let _state = state();
    css::debug::set_debug(!atom);
    css::atom_hoist::set_atom_hoist(atom.then_some(3));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color", 0, "tone", None,
        ))])
    });
    // When
    update(&mut sheet, &mut evidence, &items).required("update");
    // Then
    let maps = css::class_map::get_class_map();
    assert_eq!(maps.len(), 0);
    assert_eq!(sheet.properties["a"][&255][&0].len(), 2);
    assert_eq!(sheet.atom_plan, atom.then(BTreeSet::new));
}

#[test]
#[serial_test::serial]
fn unknown_base_is_rejected_before_callback_can_register_or_reserve() {
    // Given
    let _state = state();
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    sheet.add_css("unknown", "body{}");
    let before = capture(&sheet, &evidence);
    let mut called = false;
    // When
    let result: Result<(), UpdateError<()>> = CounterSheet::new(&mut sheet, &mut evidence)
        .with_attempt(|_| {
            called = true;
            Err(UpdateError::Output(()))
        });
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Coverage)));
    assert!(!called);
    assert_eq!(capture(&sheet, &evidence), before);
}
