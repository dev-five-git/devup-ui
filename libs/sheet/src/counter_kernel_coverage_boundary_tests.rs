use super::{KernelError, authentic_support::*};
use crate::StyleSheet;

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn capture_rejects_when_source_or_parent_is_absent_from_actual_originals(#[case] source: bool) {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color", "red", 0, None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("genuine candidate");
    let retained = sheet.counter_state.as_ref().required("retained");
    let candidate = retained.candidates().next().required("candidate");
    let projection = retained.projection(&super::authority::classes());
    let delivery = projection
        .delivery(&candidate.proof.emission.seed.placement)
        .required("delivery")
        .clone();
    let mut originals = css::file_map::get_original_ids();
    if source {
        originals.remove("a");
    } else {
        originals.insert("a".into(), 99);
    }
    css::file_map::set_original_ids(originals);
    let mut authority = super::FrozenAuthority::live();
    let before = authority.clone();
    // When
    let result = authority.capture(candidate, delivery);
    // Then
    assert_eq!(result, Err(KernelError::Authority));
    assert_eq!(authority, before);
}

#[test]
#[serial_test::serial]
fn manual_sheet_rejects_when_sheet_plan_disagrees_before_build() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet {
        atom_plan: Some(BTreeSet::new()),
        ..StyleSheet::default()
    };
    let before = capture(&sheet);
    let mut called = false;
    // When
    let result: Result<(), UpdateError<std::convert::Infallible>> = CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            called = true;
            attempt
                .prepare(
                    &styles([]),
                    UpdateRequest {
                        raw_source: "a",
                        single_css: false,
                    },
                )?
                .finish(|_, _| Ok(()))
        });
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
    assert_eq!(
        result.err().required("plan rejection").to_string(),
        "dormant counter kernel: Authority"
    );
    assert!(!called);
    assert_eq!(capture(&sheet), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn retained_sheet_rejects_when_sheet_or_global_plan_changes(#[case] global: bool) {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty retained attempt");
    if global {
        css::atom_hoist::restore_atom_plan(Some(BTreeSet::new()));
    } else {
        sheet.atom_plan = Some(BTreeSet::new());
    }
    let before = capture(&sheet);
    // When
    let result = update(&mut sheet, &styles([]));
    // Then
    assert_eq!(result, Err(UpdateError::Kernel(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[test]
#[serial_test::serial]
fn preparation_rejects_when_sheet_plan_disagrees_with_frozen_plan() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let base = super::state_live::validate(&sheet).required("valid base");
    sheet.atom_plan = Some(BTreeSet::new());
    let items = styles([]);
    let before = capture(&sheet);
    // When
    let result = super::prepare::prepare(
        &mut sheet,
        super::prepare::Preparation {
            base: &base,
            styles: &items,
            request: UpdateRequest {
                raw_source: "a",
                single_css: false,
            },
        },
    );
    // Then
    assert!(matches!(result, Err(KernelError::Authority)));
    assert_eq!(capture(&sheet), before);
}

#[test]
#[serial_test::serial]
fn output_error_preserves_diagnostic_and_rolls_back_real_reservations() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color", 0, "tone", None,
        ))])
    });
    let mut sheet = StyleSheet::default();
    let before = capture(&sheet);
    // When
    let result = CounterSheet::new(&mut sheet).with_attempt(|attempt| {
        attempt
            .prepare(
                &items,
                UpdateRequest {
                    raw_source: "a",
                    single_css: false,
                },
            )?
            .finish(|_, _| Err::<(), _>(std::io::Error::other("output sink rejected")))
    });
    // Then
    let error = result.err().required("output rejection");
    assert_eq!(error.to_string(), "output sink rejected");
    assert!(
        matches!(error, UpdateError::Output(cause) if cause.kind() == std::io::ErrorKind::Other)
    );
    assert_eq!(capture(&sheet), before);
}
