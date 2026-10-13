use super::production::application;
use super::{KernelError, authentic_support::*};
use crate::counter_evidence::ReplayError;
use crate::{StyleSheet, emission_seed::NumericSite};
use css::{Site, sparse_site::SourceFile};
use extractor::compiler_policy::{CompilerInput, CounterCompileError, with_counter_extract};
use extractor::extract_style::{CounterProducerError, compiler_receipts::ProducedData};
use std::rc::Rc;

struct Keys(Vec<String>);
impl Drop for Keys {
    fn drop(&mut self) {
        css::theme_tokens::set_typography_keys(std::mem::take(&mut self.0));
    }
}

#[rstest::rstest]
#[case(ReplayError::Allocation, KernelError::Authority)]
#[case(ReplayError::Expansion, KernelError::Coverage)]
#[case(ReplayError::Cleanup, KernelError::Cleanup)]
#[case(ReplayError::MissingPreset, KernelError::Preset)]
#[case(ReplayError::Preset, KernelError::Preset)]
fn replay_failure_keeps_its_boundary_category_when_converted(
    #[case] replay: ReplayError,
    #[case] expected: KernelError,
) {
    // Given: an independent replay rejection.
    // When
    let error = KernelError::from(replay);
    // Then
    assert_eq!(error, expected);
}

#[test]
fn missing_fixture_panics_with_context_when_option_is_none() {
    // Given
    let fixture: Option<u8> = None;
    // When
    let failure = std::panic::catch_unwind(|| fixture.required("absent fixture"));
    // Then
    let payload = failure.err().required("missing fixture must panic");
    assert_eq!(
        payload.downcast_ref::<String>().required("panic text"),
        "missing test fixture: absent fixture"
    );
}

#[test]
fn failed_fixture_panics_with_context_and_error_when_result_is_err() {
    // Given
    let fixture: Result<u8, KernelError> = Err(KernelError::Coverage);
    // When
    let failure = std::panic::catch_unwind(|| fixture.required("rejected fixture"));
    // Then
    let payload = failure.err().required("failed fixture must panic");
    assert_eq!(
        payload.downcast_ref::<String>().required("panic text"),
        "rejected fixture: Coverage"
    );
}

#[test]
#[serial_test::serial]
fn saved_presets_restore_populated_sparse_registry_when_guard_drops() {
    // Given
    let _state = state();
    let _keys = Keys(css::theme_tokens::get_typography_keys());
    let _original = Presets::save();
    let original = BTreeMap::from([(
        "coverageHeading".into(),
        vec![
            (0, "font-size".into(), "16px".into()),
            (2, "font-weight".into(), "700".into()),
        ],
    )]);
    css::content_typography::set(original.clone());
    css::theme_tokens::set_typography_keys(vec!["coverageHeading".into()]);
    let saved = Presets::save();
    css::content_typography::set(BTreeMap::from([(
        "other".into(),
        vec![(0, "font-size".into(), "24px".into())],
    )]));
    // When
    drop(saved);
    // Then
    assert_eq!(
        css::theme_tokens::get_typography_keys(),
        vec!["coverageHeading".to_string()]
    );
    assert_eq!(
        css::content_typography::declarations("coverageHeading", 0),
        original["coverageHeading"]
    );
}

#[rstest::rstest]
#[case(None, Ok(None))]
#[case(
    Some(SourceFile::D9(7)),
    Ok(Some(NumericSite {
        source: 7,
        at: 23,
        role: 5,
    }))
)]
#[case(
    Some(SourceFile::Unnumbered("received source".into())),
    Err(KernelError::Producer(CounterProducerError::UnnumberedSite))
)]
fn numeric_site_preserves_identity_or_rejects_when_raw_site_crosses_capture(
    #[case] file: Option<SourceFile>,
    #[case] expected: Result<Option<NumericSite>, KernelError>,
) {
    // Given
    let site = file.map(|file| Site {
        file,
        at: 23,
        role: 5,
    });
    // When
    let result = super::capture::numeric_site(site.as_ref());
    // Then
    assert_eq!(result, expected);
}

#[test]
#[serial_test::serial]
fn references_retain_original_when_constructor_dynamic_has_no_site() {
    css::admission::with_admission(|| {
        // Given
        let _state = state();
        let value = fixture("a", || {
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "tone", None))
        });
        assert!(matches!(&value, ExtractStyleValue::Dynamic(style) if style.site().is_none()));
        let sheet = StyleSheet::default();
        let mut retained = BTreeMap::from([("prior".into(), 91)]);
        let mut expected = retained.clone();
        expected.insert("a".into(), css::file_map::get_original_ids()["a"]);
        let before = (capture(&sheet), css::file_map::get_canonical_map());
        let config = super::authority::config();
        let threshold = css::atom_hoist::atom_hoist_threshold();
        // When
        let result = super::prepare::references(&value, &mut retained);
        // Then
        assert_eq!(result, Ok(()));
        assert_eq!(retained, expected);
        assert_eq!(
            (capture(&sheet), css::file_map::get_canonical_map()),
            before
        );
        assert_eq!(super::authority::config(), config);
        assert_eq!(css::atom_hoist::atom_hoist_threshold(), threshold);
    });
}

#[test]
#[serial_test::serial]
fn acquisition_rejects_when_operand_and_real_receipt_shapes_disagree() {
    css::admission::with_admission(|| {
        // Given
        let _state = state();
        let operand = fixture("a", || {
            ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None))
        });
        let dynamic = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
        let receipt = ProducedData::Dynamic(Rc::new(
            dynamic
                .counter_produce(None)
                .required("real dynamic acquisition"),
        ));
        let sheet = StyleSheet::default();
        let before = (capture(&sheet), css::file_map::get_canonical_map());
        let config = super::authority::config();
        let threshold = css::atom_hoist::atom_hoist_threshold();
        // When
        let result = application::acquired(&operand, &receipt);
        // Then
        assert!(matches!(result, Err(KernelError::Authority)));
        assert_eq!(
            (capture(&sheet), css::file_map::get_canonical_map()),
            before
        );
        assert_eq!(super::authority::config(), config);
        assert_eq!(css::atom_hoist::atom_hoist_threshold(), threshold);
    });
}

#[test]
#[serial_test::serial]
fn application_rejects_when_constructor_variable_allocation_is_missing() {
    css::admission::with_admission(|| {
        // Given
        let _state = state();
        let style = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
        let mut receipt = style
            .counter_produce(None)
            .required("real constructor acquisition");
        assert_eq!(style.site(), None);
        receipt
            .variable_allocation
            .take()
            .required("genuine variable allocation");
        let sheet = StyleSheet::default();
        let placement = application::placement("a", None);
        let before = (capture(&sheet), css::file_map::get_canonical_map());
        let config = super::authority::config();
        let threshold = css::atom_hoist::atom_hoist_threshold();
        // When
        let result = application::producer(
            application::Acquisition::Dynamic(&style, &receipt),
            &sheet.theme,
            &placement,
        );
        // Then
        assert_eq!(result, Err(KernelError::Authority));
        assert_eq!(
            (capture(&sheet), css::file_map::get_canonical_map()),
            before
        );
        assert_eq!(super::authority::config(), config);
        assert_eq!(css::atom_hoist::atom_hoist_threshold(), threshold);
    });
}

#[test]
#[serial_test::serial]
fn variable_rejects_when_real_use_receives_non_dynamic_acquisition() {
    css::admission::with_admission(|| {
        // Given: retain a genuine use through an aborting observation, never a published attempt.
        let _state = state();
        let style = fixture("a.tsx", || ExtractStaticStyle::new("color", "red", 0, None));
        let receipt = ProducedData::Allocation(Rc::new(
            style
                .counter_produce(None)
                .required("real static acquisition"),
        ));
        let sheet = StyleSheet::default();
        let before = (capture(&sheet), css::file_map::get_canonical_map());
        let config = super::authority::config();
        let threshold = css::atom_hoist::atom_hoist_threshold();
        let usage = match with_counter_extract(
            CompilerInput {
                filename: "a.tsx",
                code: "import {Box} from '@devup-ui/react'; const a = <Box color={tone}/>;",
                option: extractor::ExtractOption::default(),
                source_map: false,
                resolver: None,
            },
            |_, batch| {
                batch.consume(|view| {
                    Err::<(), _>(Box::new(
                        view.associations
                            .variables
                            .first()
                            .required("real variable use")
                            .clone(),
                    ))
                })
            },
        ) {
            Err(CounterCompileError::Consumer(usage)) => usage,
            Err(CounterCompileError::Compile(error)) => panic!("compiler observation: {error}"),
            Ok(()) => panic!("variable observation must abort"),
        };
        // When
        let result = super::live::receipts::variable(&usage, &[receipt]);
        // Then: neither the observation nor the validator publishes any state.
        assert_eq!(result, Err(KernelError::Authority));
        assert_eq!(
            (capture(&sheet), css::file_map::get_canonical_map()),
            before
        );
        assert_eq!(super::authority::config(), config);
        assert_eq!(css::atom_hoist::atom_hoist_threshold(), threshold);
    });
}
