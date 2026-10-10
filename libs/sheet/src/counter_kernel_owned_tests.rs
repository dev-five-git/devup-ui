use super::{
    KernelError,
    authentic_support::*,
    state_live,
    validation::{self, ValidationInput},
};
use crate::StyleSheet;

#[test]
#[serial_test::serial]
fn borrowed_facade_does_not_adopt_when_sheet_has_not_emitted_counter_output() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    // When
    let _facade = CounterSheet::new(&mut sheet);
    // Then
    assert_eq!(sheet.counter_state, None);
}

#[test]
#[serial_test::serial]
fn owned_state_moves_and_stays_private_when_empty_update_commits() {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(0));
    css::class_map::set_class_map(HashMap::from([(
        "unused".into(),
        HashMap::from([("reservation".into(), 17)]),
    )]));
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty commit");
    let expected = sheet.counter_state.clone();
    // When
    let mut moved = sheet;
    let result = update(&mut moved, &styles([]));
    // Then
    assert!(result.is_ok());
    assert_eq!(moved.counter_state, expected);
    assert!(expected.is_some());
    assert_eq!(expected.as_ref().required("owned state").threshold, None);
    let encoded = serde_json::to_value(moved.export_snapshot()).required("cache4 export");
    assert_eq!(encoded["atomNamingVersion"], 4);
    assert!(encoded.get("counter_state").is_none());
    let decoded: StyleSheet = serde_json::from_value(encoded).required("cache4 decode");
    assert_eq!(decoded.counter_state, None);
    assert!(!format!("{moved:?}").contains("CounterState"));
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn plan_membership_does_not_hoist_when_single_order_zero_or_keyframe_exception_applies(
    #[case] kind: u8,
) {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(2));
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["a".into()])));
    let items = fixture("a", || {
        let mut declaration = ExtractStaticStyle::new("color", "red", 0, None);
        match kind {
            0 => styles([ExtractStyleValue::Static(declaration)]),
            1 => {
                declaration.style_order = Some(0);
                styles([ExtractStyleValue::Static(declaration)])
            }
            2 => styles([ExtractStyleValue::Keyframes(ExtractKeyframes::default())]),
            _ => unreachable!("case"),
        }
    });
    let mut sheet = StyleSheet::default();
    // When
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: kind == 0,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .required("exception update");
    // Then
    let state = sheet.counter_state.as_ref().required("state");
    assert!(
        state
            .candidates()
            .all(|candidate| !candidate.proof.emission.seed.placement.hoisted)
    );
    state_live::capture_owned(&sheet).required("exception capture");
}

#[test]
#[serial_test::serial]
fn pure_linkage_uses_uninstalled_maps_when_live_resolver_and_registries_are_fresh() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("capture");
    let mut maps = state_live::maps();
    maps.classes
        .entry("unrelated".into())
        .or_default()
        .insert("reservation".into(), 17);
    maps.files.insert("unused".into(), 19);
    maps.originals.insert("unused".into(), 23);
    let build = state_live::build();
    css::class_map::reset_class_map();
    css::file_map::set_original_ids(BTreeMap::new());
    css::file_map::set_file_map(Default::default());
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "fresh".into())]));
    sheet.theme.breakpoints = vec![0, 999];
    let before = capture(&sheet);
    // When
    let result = validation::validate(ValidationInput {
        sheet: &sheet,
        state: sheet.counter_state.as_ref().required("state"),
        maps: &maps,
        build: &build,
    });
    // Then
    assert!(result.is_ok());
    assert_eq!(capture(&sheet), before);
    assert_eq!(sheet.properties["a"][&255][&0].len(), 1);
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn explicit_map_checks_reject_when_independent_configuration_or_binding_is_damaged(
    #[case] damage: u8,
) {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("capture");
    let mut maps = state_live::maps();
    let mut build = state_live::build();
    match damage {
        0 => build.threshold = Some(2),
        1 => build.config.prefix = "fresh".into(),
        2 => {
            *maps
                .classes
                .get_mut("D9-0")
                .required("namespace")
                .values_mut()
                .next()
                .required("slot") = 55;
        }
        3 => {
            maps.originals.insert("a".into(), 55);
        }
        4 => {
            maps.files.insert("a".into(), 55);
        }
        5 => {
            let slots = sheet
                .counter_state
                .as_mut()
                .required("state")
                .counters
                .get_mut("D9-0")
                .required("namespace");
            let witnesses = slots.remove(&0).required("slot");
            slots.insert(55, witnesses);
        }
        _ => unreachable!("case"),
    }
    let before = capture(&sheet);
    // When
    let result = validation::validate(ValidationInput {
        sheet: &sheet,
        state: sheet.counter_state.as_ref().required("state"),
        maps: &maps,
        build: &build,
    });
    // Then
    assert!(matches!(result, Err(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[test]
#[serial_test::serial]
fn frozen_bucket_hoist_rejects_when_independent_supplied_plan_disagrees() {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(2));
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["old".into()])));
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "old".into())]));
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("hoisted capture");
    let maps = state_live::maps();
    let mut build = state_live::build();
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "fresh".into())]));
    sheet.atom_plan = Some(BTreeSet::new());
    build.atom_plan.clone_from(&sheet.atom_plan);
    // When
    let result = validation::validate(ValidationInput {
        sheet: &sheet,
        state: sheet.counter_state.as_ref().required("state"),
        maps: &maps,
        build: &build,
    });
    // Then
    assert!(matches!(result, Err(KernelError::Authority)));
}

#[test]
#[serial_test::serial]
fn threshold_change_rejects_when_atom_mode_itself_is_unchanged() {
    // Given
    let _state = state();
    css::atom_hoist::set_atom_hoist(Some(2));
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty atom capture");
    css::atom_hoist::set_atom_hoist(Some(3));
    // When
    let result = update(&mut sheet, &styles([]));
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
}

#[test]
#[serial_test::serial]
fn same_proof_with_distinct_lineage_survives_when_order_zero_shares_allocation() {
    // Given
    let _state = state();
    let first = fixture("a", || {
        let mut style = ExtractStaticStyle::new("color", "red", 0, None);
        style.style_order = Some(0);
        styles([ExtractStyleValue::Static(style)])
    });
    let second = fixture("b", || {
        let mut style = ExtractStaticStyle::new("color", "red", 0, None);
        style.style_order = Some(0);
        styles([ExtractStyleValue::Static(style)])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &first).required("first lineage");
    // When
    update(&mut sheet, &second).required("second lineage");
    // Then
    let witnesses: Vec<_> = sheet
        .counter_state
        .as_ref()
        .required("state")
        .candidates()
        .collect();
    assert_eq!(witnesses.len(), 2);
    assert_eq!(witnesses[0].proof, witnesses[1].proof);
    assert_ne!(witnesses[0].lineage, witnesses[1].lineage);
}
