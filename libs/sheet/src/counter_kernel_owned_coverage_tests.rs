use super::{
    KernelError,
    authentic_support::*,
    emission,
    state::Rejection,
    state_live,
    validation::{self, ValidationInput},
};
use crate::StyleSheet;
use css::{counter_names::NameAddress, style_selector::StyleSelector};

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn supplied_group_rejects_when_genuine_receipt_is_moved_to_wrong_address(#[case] damage: u8) {
    // Given
    let _guard = state();
    css::debug::set_debug(damage == 1);
    let items = fixture("a", || {
        let mut declaration = ExtractStaticStyle::new("color", "red", 0, None);
        if damage == 1 {
            declaration.style_order = Some(0);
        }
        styles([ExtractStyleValue::Static(declaration)])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("genuine receipt");
    let maps = state_live::maps();
    let build = state_live::build();
    let owned = sheet.counter_state.as_ref().required("owned state");
    validation::validate(ValidationInput {
        sheet: &sheet,
        state: owned,
        maps: &maps,
        build: &build,
    })
    .required("valid grouping control");
    let address = &owned
        .candidates()
        .next()
        .required("receipt")
        .proof
        .allocation
        .allocation
        .address;
    match damage {
        1 => assert!(matches!(address, NameAddress::Baseline { .. })),
        0 | 2 | 3 => assert!(
            matches!(address, NameAddress::Counter { namespace, slot: 0, .. } if namespace == "D9-0")
        ),
        _ => unreachable!("case"),
    }
    let owned = sheet.counter_state.as_mut().required("owned state");
    match damage {
        0 => {
            let candidates = owned.counters.remove("D9-0").required("counter group");
            owned.baseline.extend(candidates.into_values().flatten());
        }
        1 => {
            owned.counters.insert(
                "D9-0".into(),
                BTreeMap::from([(0, std::mem::take(&mut owned.baseline))]),
            );
        }
        2 => {
            let candidates = owned.counters.remove("D9-0").required("counter group");
            owned.counters.insert("wrong".into(), candidates);
        }
        3 => {
            let slots = owned.counters.get_mut("D9-0").required("counter group");
            let candidates = slots.remove(&0).required("slot");
            slots.insert(55, candidates);
        }
        _ => unreachable!("case"),
    }
    let before = capture(&sheet);
    let maps_before = (
        maps.classes.clone(),
        maps.files.clone(),
        maps.originals.clone(),
    );
    // When
    let result = validation::validate(ValidationInput {
        sheet: &sheet,
        state: sheet.counter_state.as_ref().required("owned state"),
        maps: &maps,
        build: &build,
    });
    // Then
    assert!(matches!(result, Err(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
    assert_eq!((maps.classes, maps.files, maps.originals), maps_before);
}

fn literal_effect(sheet: &mut StyleSheet, kind: u8) -> Option<bool> {
    match kind {
        0 => Some(sheet.add_css("literal", "body{}")),
        1 => {
            sheet.add_import("literal", "https://example.test/literal.css");
            None
        }
        2 => Some(sheet.rm_global_css("a", false)),
        _ => unreachable!("case"),
    }
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn literal_grants_no_authority_when_real_base_binding_is_invalid(#[case] kind: u8) {
    // Given
    let _guard = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("genuine owned base");
    state_live::capture_owned(&sheet).required("valid base control");
    let originals = css::file_map::get_original_ids();
    css::file_map::set_original_ids(BTreeMap::from([("a".into(), 55)]));
    let mut expected = emission::clone_sheet(&sheet);
    expected.atom_plan.clone_from(&sheet.atom_plan);
    let expected_effect = literal_effect(&mut expected, kind);
    assert_eq!(expected_effect, if kind == 1 { None } else { Some(true) });
    let mut rejected = sheet.counter_state.clone().required("owned state");
    rejected.rejection = Some(Rejection(KernelError::Authority));
    expected.counter_state = Some(rejected);
    let expected = capture(&expected);
    // When
    let effect = literal_effect(&mut sheet, kind);
    // Then
    assert_eq!(effect, expected_effect);
    assert_eq!(capture(&sheet), expected);
    assert_eq!(
        sheet.counter_state.as_ref().required("state").authored,
        vec![]
    );
    assert_eq!(
        sheet.counter_state.as_ref().required("state").cleanups,
        vec![]
    );
    css::file_map::set_original_ids(originals);
    assert!(matches!(
        state_live::capture_owned(&sheet),
        Err(KernelError::Authority)
    ));
    let before = capture(&sheet);
    assert_eq!(
        update(&mut sheet, &styles([])),
        Err(UpdateError::Kernel(KernelError::Authority))
    );
    assert_eq!(capture(&sheet), before);
}
