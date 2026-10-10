use super::{KernelError, authentic_support::*, state_live};
use crate::{StyleSheet, counter_evidence::RecordFootprint};
use css::style_selector::StyleSelector;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn literal_mutations_are_owned_when_valid_counter_state_exists(#[case] kind: u8) {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty state");
    // When
    match kind {
        0 => {
            sheet.add_property("literal", "color", 1, "red", None, Some(0), Some("a"));
        }
        1 => {
            sheet.add_property_with_layer(
                "literal",
                "color",
                2,
                "red",
                Some(&StyleSelector::Global("body".into(), "a".into())),
                Some(3),
                Some("a"),
                Some("layer"),
            );
        }
        2 => {
            assert!(!sheet.add_keyframes("literal", BTreeMap::new(), Some("a")));
        }
        3 => {
            sheet.add_css("a", "body{}");
        }
        4 => sheet.add_import("a", "https://example.test/a.css"),
        5 => sheet.add_font_face(
            "a",
            &BTreeMap::from([("font-family".into(), "literal".into())]),
        ),
        _ => unreachable!("case"),
    }
    // Then
    state_live::capture_owned(&sheet).required("checked capture");
    let state = sheet.counter_state.as_ref().required("state");
    assert_eq!(state.authored.len(), 1);
    if kind == 1 {
        assert!(
            matches!(&state.authored[0], RecordFootprint::Property { bucket, order: 3, level: 2, record }
            if bucket == "a" && record.layer.as_deref() == Some("layer")
            && !record.hoisted && !record.typography && !record.owner_reset)
        );
    }
}

#[test]
#[serial_test::serial]
fn duplicate_literal_reregisters_owner_when_frozen_cleanup_leaves_outside_bucket_record() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty state");
    let selector = StyleSelector::Global("body".into(), "a".into());
    sheet.add_property(
        "literal",
        "color",
        0,
        "red",
        Some(&selector),
        None,
        Some("outside"),
    );
    assert!(sheet.rm_global_css("a", false));
    let cleanup_count = sheet
        .counter_state
        .as_ref()
        .required("state")
        .cleanups
        .len();
    // When
    let added = sheet.add_property(
        "literal",
        "color",
        0,
        "red",
        Some(&selector),
        None,
        Some("outside"),
    );
    // Then
    assert!(!added);
    assert_eq!(sheet.global_css_files, BTreeSet::from(["a".into()]));
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("state")
            .cleanups
            .len(),
        cleanup_count
    );
    state_live::capture_owned(&sheet).required("owner phase capture");
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn literal_property_captures_final_hoist_flags_when_plan_and_order_are_applied(
    #[case] order_zero: bool,
) {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(2));
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["a".into()])));
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty atom state");
    // When
    assert!(sheet.add_property(
        "literal",
        "color",
        1,
        "red",
        None,
        order_zero.then_some(0),
        Some("a")
    ));
    // Then
    let state = sheet.counter_state.as_ref().required("state");
    assert!(
        matches!(&state.authored[0], RecordFootprint::Property { bucket, order, level: 1, record }
        if bucket == "a" && *order == if order_zero { 0 } else { 255 }
        && record.hoisted != order_zero && !record.typography && !record.owner_reset)
    );
    state_live::capture_owned(&sheet).required("final flag capture");
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn irreversible_latch_rejects_checked_capture_and_kernel_when_damage_is_later_repaired(
    #[case] cleanup: bool,
) {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty state");
    sheet
        .css
        .entry("damage".into())
        .or_default()
        .insert(crate::StyleSheetCss {
            css: "unproved{}".into(),
        });
    // When
    if cleanup {
        assert!(!sheet.rm_global_css("absent", false));
    } else {
        assert!(sheet.add_css("authored", "body{}"));
    }
    sheet.css.clear();
    sheet.global_css_files.clear();
    // Then
    let state = sheet.counter_state.as_ref().required("state");
    assert_eq!(state.authored, vec![]);
    assert_eq!(state.cleanups, vec![]);
    assert!(state.rejection.is_some());
    assert!(matches!(
        state_live::capture_owned(&sheet),
        Err(KernelError::Coverage)
    ));
    assert_eq!(
        update(&mut sheet, &styles([])),
        Err(UpdateError::Kernel(KernelError::Coverage))
    );
    let mut fresh = StyleSheet::default();
    update(&mut fresh, &styles([])).required("fresh sheet clears rejection");
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn typed_owner_restores_latch_and_complete_state_when_enclosing_scope_aborts(
    #[case] prelatched: bool,
) {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty state");
    if prelatched {
        sheet
            .css
            .entry("damage".into())
            .or_default()
            .insert(crate::StyleSheetCss {
                css: "unproved{}".into(),
            });
        sheet.add_import("a", "literal");
        sheet.css.clear();
        sheet.imports.clear();
        sheet.global_css_files.clear();
    }
    let before = capture(&sheet);
    // When
    let result = css::admission::with_admission(|| {
        let publication = super::publication::Publication::new(&mut sheet);
        css::exact_attempt::with_exclusive_attempt(|| {
            if !prelatched {
                update(publication.sheet, &styles([])).required("inner success");
            }
            publication
                .sheet
                .css
                .entry("damage".into())
                .or_default()
                .insert(crate::StyleSheetCss {
                    css: "unproved{}".into(),
                });
            publication.sheet.add_import("a", "literal");
            fixture("new", || ExtractStaticStyle::new("color", "red", 0, None))
                .counter_produce(None)
                .required("reservation");
            Err::<(), _>("outer abort")
        })
    });
    // Then
    assert_eq!(result, Err("outer abort"));
    assert_eq!(capture(&sheet), before);
}
