use super::*;
use crate::counter_kernel::{CounterSheet, UpdateRequest};

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn capture_restores_complete_data_when_bookkeeping_and_theme_change(#[case] marker: u8) {
    // Given
    let mut sheet = StyleSheet::default();
    sheet.add_property("first", "color", 0, "red", None, None, Some("raw"));
    let props = sheet.properties["raw"][&255][&0].clone();
    let mut distinct = props
        .iter()
        .next()
        .unwrap_or_else(|| panic!("record"))
        .clone();
    distinct.class_name = "second".into();
    sheet
        .properties
        .get_mut("raw")
        .unwrap_or_else(|| panic!("bucket"))
        .get_mut(&255)
        .unwrap_or_else(|| panic!("order"))
        .get_mut(&0)
        .unwrap_or_else(|| panic!("level"))
        .insert(distinct);
    sheet.add_css("raw", "body{}");
    sheet.add_import("raw", "old.css");
    sheet.add_font_face(
        "raw",
        &BTreeMap::from([("font-family".into(), "old".into())]),
    );
    sheet.add_keyframes(
        "ordered",
        BTreeMap::from([(
            "from".into(),
            vec![
                ("opacity".into(), "0".into()),
                ("opacity".into(), "1".into()),
            ],
        )]),
        Some("raw"),
    );
    sheet.source_ids.insert("sheet-only".into(), 42);
    sheet.atom_plan = Some(BTreeSet::new());
    sheet.cache_restore = match marker {
        0 => CacheRestore::Manual,
        1 => CacheRestore::Rejected,
        2 => CacheRestore::Serialized {
            classes: BTreeMap::from([("unused".into(), BTreeMap::new())]),
            files: BTreeMap::from([("raw".into(), 17)]),
        },
        _ => panic!("marker"),
    };
    let before = LiveCheckpoint::capture(&sheet);
    let checkpoint = LiveCheckpoint::capture(&sheet);
    let mut replacement = StyleSheet::default();
    replacement.theme.breakpoints = vec![17, 29];
    // When
    checkpoint.restore(&mut replacement);
    // Then
    assert_eq!(LiveCheckpoint::capture(&replacement), before);
    assert_eq!(replacement.theme.breakpoints, vec![17, 29]);
    assert_eq!(replacement.properties["raw"][&255][&0].len(), 2);
}

#[test]
#[serial_test::serial]
fn capture_distinguishes_optional_counter_state_when_empty_attempt_commits() {
    // Given
    let _guard = crate::counter_fixture_support::state();
    let mut sheet = StyleSheet::default();
    let absent = LiveCheckpoint::capture(&sheet);
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &Default::default(),
                    UpdateRequest {
                        raw_source: "a",
                        single_css: true,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .unwrap_or_else(|error| panic!("{error:?}"));
    let present = LiveCheckpoint::capture(&sheet);
    let checkpoint = LiveCheckpoint::capture(&sheet);
    sheet = StyleSheet::default();
    // When
    checkpoint.restore(&mut sheet);
    // Then
    assert_ne!(present, absent);
    assert_eq!(LiveCheckpoint::capture(&sheet), present);
}

#[test]
#[serial_test::serial]
fn capture_preserves_original_rejection_when_invalid_visible_base_is_restored() {
    // Given
    let _guard = crate::counter_fixture_support::state();
    let mut sheet = StyleSheet::default();
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &Default::default(),
                    UpdateRequest {
                        raw_source: "a",
                        single_css: true,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .unwrap_or_else(|error| panic!("{error:?}"));
    sheet.add_property("wrong", "color", 0, "red", None, None, Some("damage"));
    sheet.properties.clear();
    sheet.add_css("literal", "body{}");
    let rejection = crate::snapshot6::check_install_target(&sheet);
    assert!(rejection.is_err());
    let before = LiveCheckpoint::capture(&sheet);
    let checkpoint = LiveCheckpoint::capture(&sheet);
    let mut target = StyleSheet::default();
    // When
    checkpoint.restore(&mut target);
    // Then
    assert_eq!(LiveCheckpoint::capture(&target), before);
    assert_eq!(crate::snapshot6::check_install_target(&target), rejection);
}
