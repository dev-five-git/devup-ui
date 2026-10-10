use super::content_typography::{declarations, identity, set};
use serial_test::serial;
use std::collections::BTreeMap;

#[test]
#[serial]
fn effective_typography_merges_levels_and_yields_only_actual_declarations() {
    // Given
    set(BTreeMap::from([(
        "body".into(),
        vec![
            (0, "color".into(), "red".into()),
            (1, "color".into(), "blue".into()),
            (2, "font-size".into(), "16px".into()),
        ],
    )]));
    // When
    let merged = declarations("body", 1);
    let yielded = declarations("body|color:1,invalid,font-size:no", 1);
    // Then
    assert_eq!(
        merged,
        vec![
            (1, "color".into(), "blue".into()),
            (2, "font-size".into(), "16px".into())
        ]
    );
    assert_eq!(yielded, vec![(2, "font-size".into(), "16px".into())]);
    assert_eq!(declarations("missing", 0), vec![]);
    set(BTreeMap::new());
}

#[test]
#[serial]
fn content_identity_tracks_theme_values_not_preset_labels() {
    // Given
    set(BTreeMap::from([
        ("body".into(), vec![(0, "color".into(), "red".into())]),
        ("same".into(), vec![(0, "color".into(), "red".into())]),
    ]));
    let before = identity("body", 0);
    let equivalent = identity("same", 0);
    // When
    set(BTreeMap::from([(
        "body".into(),
        vec![(0, "color".into(), "blue".into())],
    )]));
    let after = identity("body", 0);
    // Then
    assert_eq!(before, equivalent);
    assert_ne!(before, after);
    set(BTreeMap::new());
}
