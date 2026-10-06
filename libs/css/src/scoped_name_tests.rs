use crate::{Naming, class_map, file_map, naming_root, sheet_to_classname_named};
use serial_test::serial;

#[rstest::rstest]
#[case(30)]
#[case(u32::MAX)]
#[serial]
fn d9_scope_bound_includes_the_existing_anti_ad_splice(#[case] id: u32) {
    // Given
    file_map::reset_file_map();
    crate::set_prefix(None);
    file_map::set_original_ids(std::collections::BTreeMap::from([("x.tsx".into(), id)]));
    // When
    let name = sheet_to_classname_named(
        "font-family",
        0,
        Some(&"long".repeat(1000)),
        None,
        None,
        Some("x.tsx"),
        Naming::Risky,
    );
    // Then
    assert!(name.len() <= 27, "{name}");
    assert!(name.contains("-RH"));
    if id == 30 {
        assert!(name.starts_with("a-d-"), "{name}");
    }
    file_map::reset_file_map();
}

#[test]
#[serial]
fn risky_atoms_use_d9_identity_without_taking_any_counter_slots() {
    // Given
    class_map::reset_class_map();
    file_map::reset_file_map();
    crate::set_prefix(None);
    file_map::seed_file_numbers(&["a.tsx".into(), "b.tsx".into()]);
    // When
    let names = ["a.tsx", "b.tsx"].map(|file| {
        sheet_to_classname_named(
            "color",
            0,
            Some("red"),
            None,
            None,
            Some(file),
            Naming::Risky,
        )
    });
    // Then
    assert_ne!(names[0], names[1]);
    assert_eq!(names, ["a-RLcolor-vred", "b-RLcolor-vred"]);
    assert_eq!(class_map::get_class_map().len(), 0);
    file_map::reset_file_map();
}

#[test]
#[serial]
fn long_scope_and_content_are_bounded_without_arrival_numbering() {
    // Given
    file_map::reset_file_map();
    naming_root::set_root(Some("/checkout".into()));
    crate::set_prefix(Some("du-".into()));
    let file = format!("/checkout/src/{}.tsx", "long".repeat(1000));
    let value = "content".repeat(1000);
    // When
    let name = sheet_to_classname_named(
        "font-family",
        0,
        Some(&value),
        None,
        None,
        Some(&file),
        Naming::Risky,
    );
    // Then
    assert_eq!(name.len(), 40);
    assert!(name.starts_with("du-FH"));
    assert!(name.contains("-RH"));
    assert_eq!(file_map::get_file_map().len(), 0);
    naming_root::set_root(None);
    crate::set_prefix(None);
}
