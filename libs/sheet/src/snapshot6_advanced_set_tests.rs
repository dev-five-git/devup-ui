use super::advanced_support::*;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[serial_test::serial]
fn advanced_set_duplicates_reject_at_parse_before_replay_when_missing_set_class_is_repeated(
    #[case] set: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(if set == 3 { 1 } else { 2 });
    let mut value = packet(&mut sheet);
    let path = match set {
        0 => "/atom_plan".into(),
        1 => "/evidence/placements".into(),
        2 => "/evidence/phase/Retained".into(),
        3 => "/evidence/baseline".into(),
        4 => paths(&value)
            .into_iter()
            .find(|path| {
                path.ends_with("/Typography")
                    && value
                        .pointer(path)
                        .required("node")
                        .get("members")
                        .is_some()
            })
            .map(|path| format!("{path}/members"))
            .required("typography members"),
        _ => panic!("set"),
    };
    let items = value
        .pointer_mut(&path)
        .required("set")
        .as_array_mut()
        .required("array");
    let duplicate = if set == 3 {
        items
            .iter()
            .find(|candidate| candidate["proof"]["emission"]["materialization"] == "Complete")
            .required("complete baseline Candidate")
            .clone()
    } else {
        items[0].clone()
    };
    items.push(duplicate);
    // When
    let result = parse(&serde_json::to_vec(&value).required("duplicate set"));
    // Then
    assert!(
        matches!(result, Err(EvidenceError::Schema)),
        "parse-only set {path}"
    );
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn legitimate_repeated_yields_outer_rules_and_distinct_owners_adopt_when_genuine_advanced_wire_is_preserved(
    #[case] mode: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(mode);
    let bytes = encoded(&mut sheet);
    fresh();
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    let records = &target.properties["a"][&255][&0];
    assert_eq!(
        records
            .iter()
            .filter(|record| record.class_name == "owner-equal")
            .count(),
        2
    );
    let outer = records
        .iter()
        .find_map(|record| match &record.selector {
            Some(css::style_selector::StyleSelector::At { outer, .. }) => Some(outer),
            Some(
                css::style_selector::StyleSelector::Global(_, _)
                | css::style_selector::StyleSelector::Selector(_),
            )
            | None => None,
        })
        .required("outer chain");
    assert_eq!(outer.len(), 2);
    assert_eq!(outer[0], outer[1]);
    let state = target.counter_state.as_ref().required("state");
    let yields = state
        .candidates()
        .find_map(|candidate| match &candidate.proof.emission.expansion {
            crate::counter_evidence::Expansion::Typography { yielded, .. } => Some(yielded),
            crate::counter_evidence::Expansion::Static(_)
            | crate::counter_evidence::Expansion::Dynamic { .. }
            | crate::counter_evidence::Expansion::Keyframes { .. } => None,
        })
        .required("yields");
    assert_eq!(yields.len(), 2);
    assert_eq!(yields[0], yields[1]);
    assert_eq!(state.cleanups.len(), 1);
}
