use super::{fixtures::*, records, scratch};
use crate::{
    StyleSheet,
    counter_evidence::{RecordFootprint, ReplayError},
};
use css::allocation_input::NameMode;
use rstest::rstest;

#[test]
fn scratch_inserts_when_input_is_empty_without_installing_live_fields() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let input = StyleSheet::default();
    let base = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(&input, &base, request(&batch, None)));
    // Then
    assert_eq!(output.properties["delivery.tsx"][&255][&0].len(), 1);
    assert_eq!((output.collected, output.updated_base_style), (true, false));
    assert_eq!(records::capture(&input), vec![]);
    assert_eq!(output.evidence, batch.evidence);
}

#[test]
fn scratch_signals_no_change_when_full_eq_record_already_exists() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let input = sheet(&batch);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &batch,
        request(&batch, None),
    ));
    // Then
    assert_eq!(
        (output.collected, output.updated_base_style),
        (false, false)
    );
    assert_eq!(records::coverage(&input, &batch.records), Ok(()));
}

#[test]
fn rejects_unproved_reset_when_input_contains_a_record_outside_linked_coverage() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let mut input = sheet(&batch);
    let mut forged = batch.records[0].clone();
    let RecordFootprint::Property { record, .. } = &mut forged else {
        panic!("property")
    };
    record.owner_reset = true;
    record.property = "--extra".into();
    super::emission::insert(&mut input, &forged);
    let before = records::capture(&input);
    // When
    let incoming = empty(&authority);
    let result = scratch::apply_scratch(&input, &batch, request(&incoming, None));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
    assert_eq!(records::capture(&input), before);
}

#[rstest]
#[case(NameMode::Counter, false)]
#[case(NameMode::Debug, false)]
#[case(NameMode::AtomHoist, true)]
fn raw_payload_signaling_preserves_non_collected_css_semantics(
    #[case] mode: NameMode,
    #[case] atom: bool,
) {
    // Given
    let (_, mut authority) = static_fixture(mode);
    authority.authored = vec![
        RecordFootprint::Css {
            source: "raw.tsx".into(),
            css: concat!("body", "{", "color:red", "}").into(),
        },
        RecordFootprint::Import {
            source: "raw.tsx".into(),
            url: "theme.css".into(),
        },
        RecordFootprint::FontFace {
            source: "raw.tsx".into(),
            properties: [("src".into(), "url(font.woff2)".into())].into(),
        },
    ];
    let batch = linked(&[], &authority);
    let input = StyleSheet::default();
    let base = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(&input, &base, request(&batch, None)));
    // Then
    assert_eq!((output.collected, output.updated_base_style), (false, true));
    assert_eq!(output.css["raw.tsx"].len(), 1);
    assert_eq!(output.imports["raw.tsx"].len(), 1);
    assert_eq!(output.font_faces["raw.tsx"].len(), 1);
    assert_eq!(output.global_css_files.len(), 1);
    assert_eq!(batch.config.mode == NameMode::AtomHoist, atom);
}

#[test]
fn full_eq_storage_retains_authored_properties_when_lossy_ord_would_merge_them() {
    // Given
    let (candidate, mut authority) = static_fixture(NameMode::Counter);
    let original = candidate.proof.emission.expansion.records()[0].clone();
    let mut other = original;
    let RecordFootprint::Property { record, .. } = &mut other else {
        panic!("property")
    };
    record.class_name = "authored".into();
    authority.authored.push(other);
    let batch = linked(&[candidate], &authority);
    let base = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &StyleSheet::default(),
        &base,
        request(&batch, None),
    ));
    // Then
    assert_eq!(output.properties["delivery.tsx"][&255][&0].len(), 2);
}

#[test]
fn authored_reset_rejects_when_even_explicit_authored_coverage_attempts_an_exemption() {
    // Given
    let (candidate, mut authority) = static_fixture(NameMode::Counter);
    let mut forged = candidate.proof.emission.expansion.records()[0].clone();
    let RecordFootprint::Property { record, .. } = &mut forged else {
        panic!("property")
    };
    record.owner_reset = true;
    authority.authored.push(forged);
    let evidence = evidence(&[], &authority);
    // When
    let result = super::LinkedBatch::link_captured_batch(&[], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}
