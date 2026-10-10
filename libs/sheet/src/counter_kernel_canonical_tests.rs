use super::{
    Cleanup, cleanup_tests::global_dynamic, fixtures::*, phase_fixtures::*, records, scratch,
};
use crate::{
    counter_evidence::{Materialization, RecordFootprint, ReplayError},
    emission_seed::EmissionInput,
};
use css::{
    allocation_input::{LegacyInput, NameMode},
    style_selector::StyleSelector,
};
use serial_test::serial;
use std::collections::HashMap;

struct CanonicalFixture(HashMap<String, String>);

impl CanonicalFixture {
    fn collapsed() -> Self {
        let previous = css::file_map::get_canonical_map();
        let mut mappings = previous.clone();
        mappings.insert("raw.tsx".into(), "delivery.tsx".into());
        mappings.insert("peer.tsx".into(), "delivery.tsx".into());
        css::file_map::set_canonical_map(mappings);
        Self(previous)
    }
}

impl Drop for CanonicalFixture {
    fn drop(&mut self) {
        css::file_map::set_canonical_map(std::mem::take(&mut self.0));
    }
}

fn collapsed_candidate() -> (super::Candidate, super::FrozenAuthority) {
    let (candidate, _) = global_dynamic(false);
    let mut seed = candidate.proof.emission.seed;
    seed.placement.single_css = false;
    seed.placement.bucket = "delivery.tsx".into();
    let EmissionInput::Dynamic { declaration, .. } = &mut seed.body else {
        panic!("dynamic")
    };
    declaration.selector = Some(StyleSelector::Global("body".into(), "raw.tsx".into()));
    let mut input = candidate.proof.allocation.input;
    let LegacyInput::Declaration(declaration) = &mut input else {
        panic!("declaration")
    };
    declaration.selector = Some("body".into());
    fixture(seed, input, NameMode::Counter)
}

#[test]
#[serial]
fn non_single_collapsed_cleanup_matches_raw_owner_and_preserves_peer_and_other_bucket() {
    // Given
    let _canonical = CanonicalFixture::collapsed();
    let (candidate, mut authority) = collapsed_candidate();
    let consumer = candidate.proof.emission.expansion.records()[0].clone();
    let mut peer = consumer.clone();
    let RecordFootprint::Property { record, .. } = &mut peer else {
        panic!("peer")
    };
    record.selector = Some(StyleSelector::Global("body".into(), "peer.tsx".into()));
    let mut outside = consumer.clone();
    let RecordFootprint::Property { bucket, .. } = &mut outside else {
        panic!("outside")
    };
    *bucket = "other".into();
    authority.authored = vec![peer.clone(), outside.clone()];
    let base = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    let input = sheet(&base);
    let before = records::capture(&input);
    let cleanup = Cleanup {
        source: "raw.tsx".into(),
        bucket: "delivery.tsx".into(),
        single_css: false,
    };
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &base,
        request(&incoming, Some(&cleanup)),
    ));
    // Then
    let actual = records::capture(&output_sheet(&output));
    assert!(!actual.contains(&consumer));
    assert!(actual.contains(&peer));
    assert!(actual.contains(&outside));
    assert_eq!(output.global_css_files, ["peer.tsx".into()].into());
    assert_eq!(
        output.candidates[0].proof.emission.materialization,
        Materialization::AfterGlobalCleanup {
            source: "raw.tsx".into(),
            bucket: "delivery.tsx".into(),
        }
    );
    assert_eq!(records::capture(&input), before);
    let retained = relink(&output, &authority);
    assert_eq!(
        records::coverage(&output_sheet(&output), &retained.records),
        Ok(())
    );
}

#[test]
#[serial]
fn non_single_cleanup_rejects_wrong_frozen_target_without_mutating_proved_base() {
    // Given
    let _canonical = CanonicalFixture::collapsed();
    let (candidate, authority) = collapsed_candidate();
    let base = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    let input = sheet(&base);
    let before = records::capture(&input);
    let cleanup = Cleanup {
        source: "raw.tsx".into(),
        bucket: "wrong.tsx".into(),
        single_css: false,
    };
    // When
    let result = scratch::apply_scratch(&input, &base, request(&incoming, Some(&cleanup)));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Cleanup));
    assert_eq!(records::capture(&input), before);
    assert_eq!(base.cleanups, vec![]);
}
