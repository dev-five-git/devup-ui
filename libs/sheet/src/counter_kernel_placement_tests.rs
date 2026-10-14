use super::{LinkedBatch, fixtures::*, scratch};
use crate::{StyleSheet, counter_evidence::ReplayError, emission_seed::EmissionInput};
use css::{
    allocation_input::{LegacyInput, NameMode},
    style_selector::StyleSelector,
};
use rstest::rstest;

#[rstest]
#[case(NameMode::Counter, (true, None), false)]
#[case(NameMode::Counter, (false, Some(0)), true)]
#[case(NameMode::AtomHoist, (true, None), true)]
#[case(NameMode::AtomHoist, (false, Some(2)), true)]
fn placement_preserves_default_or_base_signal_when_single_css_or_order_is_captured(
    #[case] mode: NameMode,
    #[case] placement: (bool, Option<u8>),
    #[case] expected_base: bool,
) {
    // Given
    let (single, order) = placement;
    let (candidate, _) = static_fixture(mode);
    let mut seed = candidate.proof.emission.seed;
    seed.placement.single_css = single;
    if single {
        seed.placement.bucket.clear();
    }
    let EmissionInput::Static(declaration) = &mut seed.body else {
        panic!("static")
    };
    declaration.style_order = order;
    let mut input = candidate.proof.allocation.input;
    let LegacyInput::Declaration(declaration) = &mut input else {
        panic!("declaration")
    };
    declaration.order = order;
    let (candidate, authority) = fixture(seed, input, mode);
    let batch = linked(&[candidate], &authority);
    let base = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &StyleSheet::default(),
        &base,
        request(&batch, None),
    ));
    // Then
    assert_eq!(
        (output.collected, output.updated_base_style),
        (true, expected_base)
    );
    assert_eq!(
        batch.candidates[0].proof.allocation.context.file.is_none(),
        single || order == Some(0)
    );
}

#[test]
fn atom_hoisted_placement_links_when_delivery_is_frozen_before_insertion() {
    // Given
    let (candidate, _) = static_fixture(NameMode::AtomHoist);
    let mut seed = candidate.proof.emission.seed;
    seed.placement.hoisted = true;
    let (candidate, authority) =
        fixture(seed, candidate.proof.allocation.input, NameMode::AtomHoist);
    let batch = linked(&[candidate], &authority);
    let base = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &StyleSheet::default(),
        &base,
        request(&batch, None),
    ));
    // Then
    assert!(
        output.properties["delivery.tsx"][&255][&0]
            .iter()
            .all(|record| record.hoisted)
    );
    assert!(output.updated_base_style);
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
fn selector_layer_projection_links_when_only_captured_mode_selects_encoding(
    #[case] mode: NameMode,
) {
    // Given
    let (candidate, _) = static_fixture(mode);
    let mut seed = candidate.proof.emission.seed;
    let EmissionInput::Static(declaration) = &mut seed.body else {
        panic!("static")
    };
    declaration.selector = Some(StyleSelector::Selector(" &:hover ".into()));
    declaration.layer = Some("authored".into());
    let projected = match mode {
        NameMode::Counter | NameMode::Debug => " &:hover @layer authored".into(),
        NameMode::AtomHoist => {
            css::atom_name::selector_key(declaration.selector.as_ref(), Some("authored"))
        }
    };
    let mut input = candidate.proof.allocation.input;
    let LegacyInput::Declaration(declaration) = &mut input else {
        panic!("declaration")
    };
    declaration.selector = Some(projected);
    let (candidate, authority) = fixture(seed, input, mode);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 1);
}

#[test]
fn baseline_rejects_when_address_copies_a_different_mode_even_if_name_is_unchanged() {
    // Given
    let (mut candidate, authority) = static_fixture(NameMode::AtomHoist);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let css::counter_names::NameAddress::Baseline { mode, .. } =
        &mut candidate.proof.allocation.allocation.address
    else {
        panic!("baseline")
    };
    *mode = NameMode::Debug;
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}

#[rstest]
#[case(false)]
#[case(true)]
fn placement_rejects_when_registered_seed_disagrees_with_frozen_delivery(#[case] hoisted: bool) {
    // Given
    let (mut candidate, mut authority) = static_fixture(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let placement = &mut candidate.proof.emission.seed.placement;
    if hoisted {
        placement.hoisted = true;
    } else {
        placement.bucket = "other".into();
    }
    authority.placements = vec![placement.clone()];
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}
