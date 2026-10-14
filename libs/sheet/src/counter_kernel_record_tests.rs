use super::{LinkedBatch, fixtures::*, frame_tests::frames, records};
use crate::counter_evidence::{RecordFootprint, ReplayError};
use css::allocation_input::NameMode;
use rstest::rstest;

#[test]
fn keyframes_reject_when_distinct_footprints_share_bucket_and_name() {
    // Given
    let (_, mut authority) = static_fixture(NameMode::Counter);
    authority.authored = vec![
        RecordFootprint::Keyframes {
            bucket: "delivery.tsx".into(),
            name: "frame".into(),
            steps: vec![],
        },
        RecordFootprint::Keyframes {
            bucket: "delivery.tsx".into(),
            name: "frame".into(),
            steps: vec![("from".into(), vec![("opacity".into(), "0".into())])],
        },
    ];
    let evidence = evidence(&[], &authority);
    // When
    let result = LinkedBatch::link_captured_batch(&[], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[rstest]
#[case("delivery.tsx", "other-frame")]
#[case("other.tsx", "frame")]
fn keyframes_preserve_distinct_footprints_when_bucket_or_name_differs(
    #[case] bucket: &str,
    #[case] name: &str,
) {
    // Given
    let (_, mut authority) = static_fixture(NameMode::Counter);
    authority.authored = vec![
        RecordFootprint::Keyframes {
            bucket: "delivery.tsx".into(),
            name: "frame".into(),
            steps: vec![],
        },
        RecordFootprint::Keyframes {
            bucket: bucket.into(),
            name: name.into(),
            steps: vec![("from".into(), vec![("opacity".into(), "0".into())])],
        },
    ];
    let evidence = evidence(&[], &authority);
    // When
    let batch = require_ok(LinkedBatch::link_captured_batch(&[], &evidence, &authority));
    // Then
    assert_eq!(batch.records, authority.authored);
}

#[test]
fn keyframes_deduplicate_when_footprints_are_exactly_equal() {
    // Given
    let (_, mut authority) = static_fixture(NameMode::Counter);
    let record = RecordFootprint::Keyframes {
        bucket: "delivery.tsx".into(),
        name: "frame".into(),
        steps: vec![("from".into(), vec![("opacity".into(), "0".into())])],
    };
    authority.authored = vec![record.clone(), record.clone()];
    let evidence = evidence(&[], &authority);
    // When
    let batch = require_ok(LinkedBatch::link_captured_batch(&[], &evidence, &authority));
    // Then
    assert_eq!(batch.records, vec![record]);
}

#[test]
fn capture_preserves_frame_name_order_duplicates_and_empty_steps_when_sheet_is_materialized() {
    // Given
    let (candidate, authority) = frames(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let input = sheet(&batch);
    // When
    let captured = records::capture(&input);
    // Then
    assert_eq!(captured, batch.records);
    let [
        RecordFootprint::Keyframes {
            bucket,
            name,
            steps,
        },
    ] = captured.as_slice()
    else {
        panic!("one keyframe footprint")
    };
    assert_eq!(bucket, "delivery.tsx");
    assert_eq!(name, &batch.candidates[0].proof.emission.name);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].0, "from");
    assert_eq!(steps[0].1.len(), 3);
    assert_eq!(steps[0].1[0], ("opacity".into(), "0".into()));
    assert_eq!(steps[0].1[1], steps[0].1[0]);
    assert_eq!(steps[0].1[2].0, "typography");
    assert_eq!(steps[1], ("to".into(), vec![]));
}

#[test]
fn capture_preserves_authored_font_face_source_and_properties_when_sheet_is_materialized() {
    // Given
    let (_, mut authority) = static_fixture(NameMode::Counter);
    let font = RecordFootprint::FontFace {
        source: "fonts.tsx".into(),
        properties: [
            ("font-family".into(), "Fixture Face".into()),
            ("src".into(), "url(fixture.woff2)".into()),
        ]
        .into(),
    };
    authority.authored = vec![font.clone()];
    let batch = linked(&[], &authority);
    let input = sheet(&batch);
    // When
    let captured = records::capture(&input);
    // Then
    assert_eq!(
        captured,
        vec![
            font,
            RecordFootprint::GlobalCssOwner {
                source: "fonts.tsx".into()
            }
        ]
    );
}
