use super::{KernelError, authentic_support::*};
use crate::StyleSheet;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn complete_state_rolls_back_when_output_fails_panics_abandons_or_outer_aborts(
    #[case] failure: u8,
) {
    // Given
    let _state = state();
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["old".into()])));
    css::class_map::set_class_map(HashMap::from([("empty".into(), HashMap::new())]));
    let mut sheet = StyleSheet {
        cache_restore: crate::cache_snapshot::CacheRestore::Rejected,
        ..StyleSheet::default()
    };
    sheet.source_ids.insert("unrelated".into(), 17);
    let before = capture(&sheet);
    // When
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CounterSheet::new(&mut sheet).with_attempt(|attempt| {
            fixture("a", || {
                let items = styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                    "color", 0, "tone", None,
                ))]);
                let prepared = attempt.prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: true,
                    },
                )?;
                match failure {
                    0 => prepared.finish(|_, _| Err::<(), _>("output")),
                    1 => prepared.finish(|_, _| -> Result<(), &str> { panic!("output unwind") }),
                    2 => {
                        drop(prepared);
                        Err(UpdateError::Output("abandoned"))
                    }
                    3 => {
                        let completed = prepared.finish(|_, _| Ok::<_, &str>(()))?;
                        let inner = css::class_map::Attempt::begin();
                        fixture("inner", || ExtractStaticStyle::new("opacity", "0", 0, None))
                            .counter_produce(None)
                            .required("inner allocation");
                        inner.commit();
                        drop(completed);
                        Err(UpdateError::Output("outer abort"))
                    }
                    _ => unreachable!("case"),
                }
            })
        })
    }));
    // Then
    match failure {
        1 => assert!(result.is_err()),
        0 | 2 | 3 => assert!(result.required("no panic").is_err()),
        _ => unreachable!("case"),
    }
    assert_eq!(capture(&sheet), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn final_boundary_rejects_map_damage_when_output_or_post_finish_code_mutates_it(
    #[case] after_finish: bool,
) {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let before = capture(&sheet);
    // When
    let result = CounterSheet::new(&mut sheet).with_attempt(|attempt| {
        fixture("a", || {
            let items = styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                "color", 0, "tone", None,
            ))]);
            let completion = attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: false,
                    },
                )?
                .finish(|_, _| {
                    if !after_finish {
                        css::class_map::reset_class_map();
                    }
                    Ok::<_, ()>(())
                })?;
            if after_finish {
                css::class_map::set_class_map(HashMap::new());
            }
            Ok(completion)
        })
    });
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[test]
#[serial_test::serial]
fn construction_damage_is_rejected_before_incoming_can_recreate_the_base_key() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("initial update");
    let before = capture(&sheet);
    // When
    let result: Result<(), UpdateError<()>> =
        CounterSheet::new(&mut sheet).with_attempt(|attempt| {
            css::class_map::reset_class_map();
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: false,
                    },
                )?
                .finish(|_, _| Ok(()))
        });
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[test]
#[serial_test::serial]
fn producer_failure_rolls_back_both_reservations_when_current_keyframes_follow_dynamic() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let current = ExtractKeyframes::default();
    let before = capture(&sheet);
    // When
    let result = CounterSheet::new(&mut sheet).with_attempt(|attempt| {
        fixture("a", || {
            let items = styles([
                ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "tone", None)),
                ExtractStyleValue::Keyframes(current),
            ]);
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: true,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
    });
    // Then
    assert_eq!(
        result,
        Err(UpdateError::Kernel(KernelError::Producer(
            extractor::extract_style::CounterProducerError::WrongPolicy
        )))
    );
    assert_eq!(capture(&sheet), before);
    let retry = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color", 0, "tone", None,
        ))])
    });
    assert!(update(&mut sheet, &retry).required("retry").collected);
}
