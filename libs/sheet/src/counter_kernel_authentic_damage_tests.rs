use super::{KernelError, authentic_support::*};
use crate::{StyleSheet, counter_evidence::Expansion};

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
fn retained_damage_rejects_before_callback_when_independent_authority_is_changed(
    #[case] damage: u8,
) {
    // Given
    let _state = state();
    let items = fixture("a", || {
        let dynamic = ExtractDynamicStyle::new("color", 0, "tone", None);
        styles([ExtractStyleValue::Dynamic(if damage == 7 {
            dynamic.at(2)
        } else {
            dynamic
        })])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("initial");
    let retained = sheet.counter_state.as_mut().required("retained");
    match damage {
        0 => {
            retained
                .candidates_mut()
                .next()
                .required("candidate")
                .lineage
                .parent = 55;
        }
        1 => retained.config.prefix = "wrong".into(),
        2 => {
            retained
                .candidates_mut()
                .next()
                .required("candidate")
                .proof
                .emission
                .seed
                .placement
                .bucket = "wrong".into();
        }
        3 => {
            retained
                .candidates_mut()
                .next()
                .required("candidate")
                .lineage
                .variable
                .as_mut()
                .required("variable")
                .original = 55;
        }
        4 => {
            retained.phase = super::BatchPhase::Retained(BTreeSet::from(["wrong".into()]));
        }
        5 => {
            let proof = &mut retained.candidates_mut().next().required("candidate").proof;
            if let Expansion::Dynamic { consumer, .. } = &mut proof.emission.expansion
                && let crate::counter_evidence::RecordFootprint::Property { record, .. } =
                    consumer.as_mut()
            {
                record.value = "wrong".into();
            }
            let records = sheet
                .properties
                .get_mut("a")
                .required("bucket")
                .get_mut(&255)
                .required("order")
                .get_mut(&0)
                .required("level");
            let mut altered = records
                .iter()
                .find(|record| !record.owner_reset)
                .required("consumer")
                .clone();
            records.remove(&altered);
            altered.value = "wrong".into();
            records.insert(altered);
        }
        6 => {
            retained
                .candidates_mut()
                .next()
                .required("candidate")
                .proof
                .allocation
                .input = css::allocation_input::LegacyInput::Keyframes("wrong".into());
        }
        7 => {
            if let crate::emission_seed::EmissionInput::Dynamic {
                site: Some(site), ..
            } = &mut retained
                .candidates_mut()
                .next()
                .required("candidate")
                .proof
                .emission
                .seed
                .body
            {
                site.role = 7;
            } else {
                panic!("numeric fixture");
            }
        }
        _ => unreachable!("case"),
    }
    let before = capture(&sheet);
    let mut called = false;
    // When
    let result: Result<(), UpdateError<()>> = CounterSheet::new(&mut sheet).with_attempt(|_| {
        called = true;
        Err(UpdateError::Output(()))
    });
    // Then
    assert!(matches!(result, Err(UpdateError::Kernel(_))));
    assert!(!called);
    assert_eq!(capture(&sheet), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn actual_binding_damage_rejects_when_referenced_file_or_original_ordinal_changes(
    #[case] original: bool,
) {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("initial");
    if original {
        css::file_map::set_original_ids(BTreeMap::from([("a".into(), 7)]));
    } else {
        let mut files = css::file_map::get_file_map();
        files.insert("a".into(), 7);
        css::file_map::set_file_map(files);
    }
    let before = capture(&sheet);
    // When
    let result = update(&mut sheet, &styles([]));
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn cleanup_receipt_or_fresh_phase_damage_rejects_when_real_partial_survival_is_retained(
    #[case] fresh: bool,
) {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(css::style_selector::StyleSelector::Global(
                "body".into(),
                "a".into(),
            )),
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("initial");
    update(&mut sheet, &styles([])).required("real partial cleanup");
    let retained = sheet.counter_state.as_mut().required("retained");
    if fresh {
        retained.phase = super::BatchPhase::Fresh;
    } else {
        retained.cleanups[0].bucket = "wrong".into();
    }
    let before = capture(&sheet);
    // When
    let result = update(&mut sheet, &styles([]));
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Cleanup)));
    assert_eq!(capture(&sheet), before);
}
