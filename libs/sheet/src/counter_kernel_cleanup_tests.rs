use super::{Cleanup, dynamic_fixture::dynamic, fixtures::*, records, scratch};
use crate::{
    counter_evidence::{Materialization, RecordFootprint},
    emission_seed::EmissionInput,
};
use css::{
    allocation_input::{LegacyInput, NameMode},
    style_selector::{AtRuleKind, StyleSelector},
};
use rstest::rstest;
use serial_test::serial;

pub(super) fn global_dynamic(at: bool) -> (super::Candidate, super::FrozenAuthority) {
    let (mut candidate, _) = dynamic(NameMode::Counter, true);
    candidate.proof.emission.seed.placement.single_css = true;
    candidate.proof.emission.seed.placement.bucket.clear();
    let selector = if at {
        StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".into(),
            selector: Some("body".into()),
            outer: vec![],
            file: Some("raw.tsx".into()),
        }
    } else {
        StyleSelector::Global("body".into(), "raw.tsx".into())
    };
    let EmissionInput::Dynamic { declaration, .. } = &mut candidate.proof.emission.seed.body else {
        panic!("dynamic")
    };
    declaration.selector = Some(selector.clone());
    let LegacyInput::Declaration(input) = &mut candidate.proof.allocation.input else {
        panic!("declaration")
    };
    input.selector = Some(selector.as_class_str().into_owned());
    fixture(
        candidate.proof.emission.seed,
        candidate.proof.allocation.input,
        NameMode::Counter,
    )
}

pub(super) fn cleanup() -> Cleanup {
    Cleanup {
        source: "raw.tsx".into(),
        bucket: String::new(),
        single_css: true,
    }
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn guarded_cleanup_keeps_exact_plain_reset_when_global_or_at_consumer_disappears(#[case] at: bool) {
    // Given
    let (candidate, authority) = global_dynamic(at);
    let batch = linked(&[candidate], &authority);
    let input = sheet(&batch);
    let before = records::capture(&input);
    let incoming = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert!(output.cleaned);
    assert_eq!(output.global_css_files.len(), 0);
    assert_eq!(output.properties[""][&255][&0].len(), 1);
    let reset = output.properties[""][&255][&0]
        .iter()
        .next()
        .unwrap_or_else(|| panic!("reset"));
    assert_eq!(
        (
            &reset.selector,
            &reset.layer,
            reset.value.as_str(),
            reset.owner_reset
        ),
        (&None, &None, "initial", true)
    );
    assert_eq!(
        output.candidates[0].proof.emission.materialization,
        Materialization::AfterGlobalCleanup {
            source: "raw.tsx".into(),
            bucket: String::new()
        }
    );
    assert_eq!(records::capture(&input), before);
    assert_eq!(
        batch.evidence.counters[""][&4][0].emission.materialization,
        Materialization::Complete
    );
}

#[test]
#[serial]
fn cleanup_guard_false_retains_evidence_when_owner_is_unregistered() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let input = sheet(&batch);
    let incoming = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert!(!output.cleaned);
    assert_eq!(output.evidence, batch.evidence);
    assert_eq!(output.cleanups, vec![]);
}

#[test]
#[serial]
fn cleanup_drops_all_witnesses_when_global_typography_members_are_removed() {
    // Given
    let (mut candidate, _) = super::value_tests::typography();
    candidate.proof.emission.seed.placement.single_css = true;
    candidate.proof.emission.seed.placement.bucket.clear();
    let EmissionInput::Typography(declaration) = &mut candidate.proof.emission.seed.body else {
        panic!("typography")
    };
    declaration.selector = Some(StyleSelector::Global("body".into(), "raw.tsx".into()));
    let LegacyInput::Declaration(input) = &mut candidate.proof.allocation.input else {
        panic!("declaration")
    };
    input.selector = Some("body".into());
    let (candidate, authority) = fixture(
        candidate.proof.emission.seed,
        candidate.proof.allocation.input,
        NameMode::Counter,
    );
    let batch = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &sheet(&batch),
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert_eq!(output.evidence.counters.len(), 0);
    assert_eq!(output.candidates, vec![]);
    assert_eq!(output.global_css_files.len(), 0);
}

#[test]
#[serial]
fn owner_registration_is_pruned_when_other_bucket_global_rule_survives() {
    // Given
    let (candidate, mut authority) = global_dynamic(false);
    let mut outside = candidate.proof.emission.expansion.records()[0].clone();
    let RecordFootprint::Property { bucket, .. } = &mut outside else {
        panic!("property")
    };
    *bucket = "other".into();
    authority.authored.push(outside);
    authority.authored.push(RecordFootprint::Import {
        source: "raw.tsx".into(),
        url: "theme.css".into(),
    });
    let batch = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &sheet(&batch),
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert_eq!(output.properties["other"][&255][&0].len(), 1);
    assert_eq!(output.global_css_files.len(), 0);
    assert_eq!(output.imports.len(), 0);
    assert_eq!(output.evidence.authored.len(), 1);
}
