use super::test_support::*;

#[rstest::rstest]
#[case(0, false)]
#[case(1, false)]
#[case(2, false)]
#[case(0, true)]
#[case(1, true)]
#[case(2, true)]
#[serial_test::serial]
fn candidate_plan_checks_preserve_exceptions_when_atom_bucket_is_in_frozen_plan(
    #[case] kind: u8,
    #[case] single: bool,
) {
    // Given
    let _guard = state();
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
            _ => panic!("kind"),
        }
    });
    let mut sheet = StyleSheet::default();
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: single,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .required("update");
    let bytes = encoded(&mut sheet);
    fresh();
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert!(
        target
            .counter_state
            .as_ref()
            .required("state")
            .candidates()
            .all(|candidate| candidate.proof.emission.seed.placement.hoisted
                == (!single && kind == 0))
    );
    assert_eq!(
        css::atom_hoist::atom_plan(),
        Some(BTreeSet::from(["a".into()]))
    );
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn baseline_and_prefix_rendering_adopts_when_genuine_debug_or_counter_output_is_exported(
    #[case] debug: bool,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    css::set_prefix(Some("ad".into()));
    css::debug::set_debug(debug);
    let mut sheet = family_sheet(1);
    let bytes = encoded(&mut sheet);
    let expected = super::super::records::capture(&sheet)
        .into_iter()
        .collect::<rustc_hash::FxHashSet<_>>();
    fresh();
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(
        super::super::records::capture(&target)
            .into_iter()
            .collect::<rustc_hash::FxHashSet<_>>(),
        expected
    );
    let state = target.counter_state.as_ref().required("state");
    assert_eq!(state.baseline.len(), usize::from(debug));
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
#[serial_test::serial]
fn placement_flags_reject_coordinated_damage_when_seed_retains_exact_record_identity(
    #[case] damage: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    let mut value = packet(&mut sheet);
    let field = match damage {
        0 => "c",
        1 => "p",
        2 => "s",
        3 => "l",
        4 => "t",
        5 => "h",
        6 => "r",
        7 => "v",
        _ => panic!("damage"),
    };
    let changed = match damage {
        0 | 1 | 3 | 7 => json!("changed"),
        2 => json!({"Global":["body","other"]}),
        4..=6 => json!(true),
        _ => panic!("damage"),
    };
    value["properties"]["a"]["255"]["0"][0][field] = changed.clone();
    witness(&mut value)["proof"]["emission"]["expansion"]["Static"][0]["Property"]["record"]
        [field] = changed;
    // When
    let result = admit(value);
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Coverage))
    ));
}

#[test]
#[serial_test::serial]
fn threshold_freshness_rejects_when_atom_mode_remains_enabled_but_threshold_changes() {
    // Given
    let _guard = state();
    css::atom_hoist::set_atom_hoist(Some(2));
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty atom");
    let certificate =
        validate_snapshot(parse(&encoded(&mut sheet)).required("parse")).required("admit");
    css::atom_hoist::set_atom_hoist(Some(3));
    let mut target = StyleSheet::default();
    let before = observe(&target);
    // When
    let result = certificate.install_into(&mut target);
    // Then
    assert_eq!(result, Err(EvidenceError::Configuration));
    assert_eq!(observe(&target), before);
}
