use super::{
    cleanup_tests::{cleanup, global_dynamic},
    fixtures::*,
    phase_fixtures::*,
    records, scratch,
};
use crate::counter_evidence::Materialization;
use css::allocation_input::NameMode;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn old_global_cleanup_preserves_old_reset_and_complete_fresh_replacement() {
    // Given
    let (old, old_authority) = global_dynamic(false);
    let (new, new_authority) = replacement();
    let old_records: Vec<_> = old
        .proof
        .emission
        .expansion
        .records()
        .into_iter()
        .cloned()
        .collect();
    let new_records: Vec<_> = new
        .proof
        .emission
        .expansion
        .records()
        .into_iter()
        .cloned()
        .collect();
    let base = linked(&[old], &old_authority);
    let incoming = linked(&[new], &new_authority);
    let input = sheet(&base);
    let before = records::capture(&input);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &base,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    let actual = records::capture(&output_sheet(&output));
    assert!(!actual.contains(&old_records[0]));
    assert!(actual.contains(&old_records[1]));
    assert!(new_records.iter().all(|record| actual.contains(record)));
    assert_eq!(
        output.candidates[0].proof.emission.materialization,
        Materialization::AfterGlobalCleanup {
            source: "raw.tsx".into(),
            bucket: String::new(),
        }
    );
    assert_eq!(
        output.candidates[1].proof.emission.materialization,
        Materialization::Complete
    );
    assert!(output.global_css_files.contains("raw.tsx"));
    assert_eq!(records::capture(&input), before);
    assert_eq!(
        base.candidates[0].proof.emission.materialization,
        Materialization::Complete
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn requested_cleanup_has_false_guard_when_empty_or_unregistered_base_receives_fresh_global(
    #[case] existing: bool,
) {
    // Given
    let (new, authority) = replacement();
    let incoming = linked(&[new], &authority);
    let (plain, plain_authority) = static_fixture(NameMode::Counter);
    let base = if existing {
        linked(&[plain], &plain_authority)
    } else {
        empty(&authority)
    };
    let input = sheet(&base);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &base,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert!(!output.cleaned);
    assert_eq!(output.cleanups, vec![]);
    assert!(output.global_css_files.contains("raw.tsx"));
    let actual = records::capture(&output_sheet(&output));
    assert!(
        incoming
            .records
            .iter()
            .all(|record| actual.contains(record))
    );
    assert!(
        output
            .candidates
            .iter()
            .all(|candidate| candidate.proof.emission.materialization == Materialization::Complete)
    );
}

#[test]
#[serial]
fn subsequent_cleanup_removes_fresh_replacement_after_retained_state_is_relinked() {
    // Given
    let (old, old_authority) = global_dynamic(false);
    let (new, new_authority) = replacement();
    let base = linked(&[old], &old_authority);
    let incoming = linked(&[new], &new_authority);
    let first = require_ok(scratch::apply_scratch(
        &sheet(&base),
        &base,
        request(&incoming, Some(&cleanup())),
    ));
    let authority = combine(&old_authority, &new_authority);
    let retained = relink(&first, &authority);
    let empty_incoming = empty(&authority);
    let input = output_sheet(&first);
    // When
    let output = require_ok(scratch::apply_scratch(
        &input,
        &retained,
        request(&empty_incoming, Some(&cleanup())),
    ));
    // Then
    assert!(output.cleaned);
    assert_eq!(output.global_css_files.len(), 0);
    assert_eq!(output.properties[""][&255][&0].len(), 2);
    assert!(
        output.properties[""][&255][&0]
            .iter()
            .all(|record| record.owner_reset && record.value == "initial")
    );
    assert!(
        output
            .candidates
            .iter()
            .all(|candidate| candidate.proof.emission.materialization
                == Materialization::AfterGlobalCleanup {
                    source: "raw.tsx".into(),
                    bucket: String::new(),
                })
    );
    assert_eq!(output.evidence.counters[""].len(), 2);
}
