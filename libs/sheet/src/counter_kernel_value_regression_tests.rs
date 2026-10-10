use super::{LinkedBatch, fixtures::*};
use crate::{
    counter_evidence::{RecordFootprint, ReplayError},
    emission_seed::{EmissionInput, Resolution},
};
use css::allocation_input::{LegacyDeclaration, LegacyInput, NameMode};
use rstest::rstest;

#[rstest]
#[case(Resolution::CssVariable)]
#[case(Resolution::FirstValue)]
fn margin_rejects_collapsed_candidate_when_baseline_keeps_authored_spacing(
    #[case] resolution: Resolution,
) {
    // Given
    let mut declaration = declaration("margin", "14px 14px");
    declaration.resolution = resolution;
    declaration.first_value = Some("24px 24px".into());
    let (candidate, authority) = fixture(
        seed(EmissionInput::Static(declaration)),
        LegacyInput::Declaration(LegacyDeclaration {
            property: "margin".into(),
            level: 0,
            value: Some("14px".into()),
            selector: None,
            order: None,
        }),
        NameMode::Counter,
    );
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}

#[test]
fn font_family_comma_value_links_when_css_variable_resolution_uses_real_multi_optimization() {
    // Given
    let (candidate, authority) = fixture(
        seed(EmissionInput::Static(declaration(
            "font-family",
            "'Roboto', sans-serif",
        ))),
        LegacyInput::Declaration(LegacyDeclaration {
            property: "font-family".into(),
            level: 0,
            value: Some("Roboto,sans-serif".into()),
            selector: None,
            order: None,
        }),
        NameMode::Counter,
    );
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    let RecordFootprint::Property { record, .. } = &batch.records[0] else {
        panic!("property")
    };
    assert_eq!(record.value, "Roboto,sans-serif");
}

#[test]
fn width_first_value_links_when_frozen_14px_overrides_raw_token() {
    // Given
    let mut declaration = declaration("width", "$size");
    declaration.resolution = Resolution::FirstValue;
    declaration.first_value = Some("14px".into());
    let (candidate, authority) = fixture(
        seed(EmissionInput::Static(declaration)),
        LegacyInput::Declaration(LegacyDeclaration {
            property: "width".into(),
            level: 0,
            value: Some("14px".into()),
            selector: None,
            order: None,
        }),
        NameMode::Counter,
    );
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    let RecordFootprint::Property { record, .. } = &batch.records[0] else {
        panic!("property")
    };
    assert_eq!(record.value, "14px");
}
