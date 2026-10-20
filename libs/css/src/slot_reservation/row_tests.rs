use super::*;
use serial_test::serial;

#[test]
#[serial]
fn rollback_removes_each_allocator_kind_but_retains_existing_and_empty_buckets() {
    // Given
    reset();
    crate::debug::set_debug(false);
    crate::class_map::reset_class_map();
    crate::file_map::reset_file_map();
    crate::class_map::with_class_map_mut(|map| {
        map.insert("empty".into(), HashMap::new());
    });
    let existing = crate::sheet_to_variable_name("existing", 0, None);
    let before = get_class_map();
    let _batch = ScratchSlots::default();
    let _ = crate::file_map::get_file_num_by_filename("seeded");
    // When
    {
        let _row = RowSlots::default();
        assert_eq!(crate::sheet_to_variable_name("existing", 0, None), existing);
        drop(crate::sheet_to_variable_name("gap", 0, None));
        drop(crate::keyframes_to_keyframes_name("spin", Some("extra")));
        drop(crate::sheet_to_classname(
            "color",
            0,
            Some("red"),
            None,
            None,
            Some("empty"),
        ));
        drop(crate::sheet_to_classname(
            "width",
            0,
            Some("1px"),
            None,
            None,
            Some("empty"),
        ));
    }
    // Then
    assert_eq!(get_class_map(), before);
    assert_eq!(get_file_map().len(), 1);
    assert_eq!(get_file_map().get_by_left("seeded"), Some(&0));
}

#[test]
#[serial]
fn committed_nested_row_remains_subject_to_outer_rollback() {
    // Given
    reset();
    crate::debug::set_debug(false);
    crate::class_map::reset_class_map();
    let batch = ScratchSlots::default();
    // When
    {
        let _outer = RowSlots::default();
        let inner = RowSlots::default();
        drop(crate::sheet_to_variable_name("nested", 0, None));
        inner.commit();
    }
    let row = RowSlots::default();
    let name = crate::sheet_to_variable_name("committed", 0, None);
    row.commit();
    batch.commit();
    // Then
    assert_eq!(name, "--a");
    assert_eq!(get_class_map()[""].len(), 1);
    assert_eq!(crate::sheet_to_variable_name("committed", 0, None), name);
    assert_eq!(unreserved_slot_count(), 0);
    reset();
}

#[test]
#[serial]
fn atom_counter_insertions_use_the_same_row_rollback() {
    // Given
    reset();
    crate::class_map::reset_class_map();
    crate::atom_hoist::set_atom_hoist(Some(2));
    let batch = ScratchSlots::default();
    let before = get_class_map();
    // When
    {
        let _row = RowSlots::default();
        drop(crate::sheet_to_variable_name("gap", 0, None));
        drop(crate::keyframes_to_keyframes_name("spin", None));
        drop(crate::sheet_to_classname(
            "color",
            0,
            Some("red"),
            None,
            None,
            None,
        ));
        assert_eq!(get_class_map()[""].len(), 3);
    }
    batch.commit();
    // Then
    assert_eq!(get_class_map(), before);
    assert_eq!(unreserved_slot_count(), 0);
    crate::atom_hoist::set_atom_hoist(None);
    reset();
}

#[test]
#[serial]
fn owner_snapshot_and_unwind_preserve_suppression_until_next_batch() {
    // Given
    reset();
    let batch = ScratchSlots::default();
    record_suppressed_row();
    batch.commit();
    record_insert();
    let saved = capture();
    reset();
    assert_eq!(suppressed_row_count(), 0);
    restore(saved);
    // When
    let result = std::panic::catch_unwind(|| {
        let _batch = ScratchSlots::default();
        record_suppressed_row();
        record_suppressed_row();
        panic!("batch unwind");
    });
    // Then
    assert!(result.is_err());
    assert_eq!(suppressed_row_count(), 1);
    assert_eq!(unreserved_slot_count(), 1);
    ScratchSlots::default().commit();
    assert_eq!(suppressed_row_count(), 0);
    assert_eq!(unreserved_slot_count(), 1);
    reset();
}
