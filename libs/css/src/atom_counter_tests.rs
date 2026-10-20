use super::*;
use serial_test::serial;
use std::collections::{HashMap, HashSet};

fn setup() {
    class_map::reset_class_map();
    file_map::reset_file_map();
    file_map::reset_canonical_map();
    atom_hoist::restore_atom_plan(None);
    atom_hoist::set_atom_hoist(Some(2));
    file_routes::set_file_routes(HashMap::from([("shared".into(), HashSet::from([0, 1]))]));
    set_prefix(None);
    debug::set_debug(false);
}

#[test]
#[serial]
fn w66_atom_symbols_share_short_global_slots_and_canonical_local_slots() {
    // Given
    setup();
    file_map::set_canonical_map(HashMap::from([("alias".into(), "private".into())]));
    // When / Then
    assert_eq!(
        sheet_to_classname("color", 0, Some("red"), None, None, Some("shared")),
        "a"
    );
    assert_eq!(sheet_to_variable_name("color", 0, None), "--b");
    assert_eq!(keyframes_to_keyframes_name("frames", None), "c");
    assert_eq!(
        sheet_to_classname("color", 0, Some("red"), None, None, Some("alias")),
        "a-a"
    );
    assert_eq!(
        sheet_to_classname("color", 0, Some("red"), None, None, Some("private")),
        "a-a"
    );
    assert_eq!(
        keyframes_to_keyframes_name("frames", Some("private")),
        "a-b"
    );
    assert_eq!(keyframes_to_keyframes_name("frames", Some("shared")), "b-a");
    assert_eq!(class_map::get_class_map()[""].len(), 3);
    assert_eq!(class_map::get_class_map()["private"].len(), 2);
    atom_hoist::set_atom_hoist(None);
    atom_hoist::restore_atom_plan(None);
    class_map::reset_class_map();
    file_map::reset_file_map();
    file_map::reset_canonical_map();
    file_routes::reset_file_routes();
}

#[test]
#[serial]
fn w66_atom_debug_uses_the_same_prefixed_counters() {
    // Given
    let build = |debug| {
        setup();
        set_prefix(Some("du-".into()));
        debug::set_debug(debug);
        // When
        (
            sheet_to_classname("color", 0, Some("red"), None, None, Some("shared")),
            sheet_to_variable_name("color", 0, None),
            keyframes_to_keyframes_name("frames", Some("private")),
        )
    };
    // Then
    let expected = ("du-a".into(), "--du-b".into(), "du-a-a".into());
    assert_eq!(build(false), expected);
    assert_eq!(build(true), expected);
    setup();
    atom_hoist::set_atom_hoist(None);
    file_routes::reset_file_routes();
}

#[test]
#[serial]
fn atom_counter_keys_preserve_value_boundaries_and_placement_identity() {
    // Given
    setup();
    // When
    let names = [
        sheet_to_classname("a-b", 0, Some("c"), None, None, None),
        sheet_to_classname("a", 0, Some("b-c"), None, None, None),
        sheet_to_classname("color", 0, None, None, None, None),
        sheet_to_classname("color", 0, Some(""), None, None, None),
        sheet_to_classname("color", 0, Some("red"), None, None, None),
        sheet_to_classname("color", 0, Some("red"), None, None, Some("shared")),
        keyframes_to_keyframes_name("color", None),
        sheet_to_variable_name("color", 0, None)
            .trim_start_matches("--")
            .to_string(),
    ];
    // Then
    assert_eq!(names, ["a", "b", "c", "d", "e", "f", "g", "h"]);
    assert_eq!(
        sheet_to_classname("color", 0, Some("red"), None, Some(0), Some("private")),
        "i"
    );
    assert_eq!(
        sheet_to_classname("color", 0, Some("red"), None, Some(0), None),
        "i"
    );
    setup();
    atom_hoist::set_atom_hoist(None);
    file_routes::reset_file_routes();
}
