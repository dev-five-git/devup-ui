use serial_test::serial;

#[test]
#[serial]
#[should_panic(expected = "class_map: a data root is already held")]
fn rejects_class_read_recursion_when_class_callback_holds_lock() {
    // Given / When / Then
    crate::class_map::with_class_map(|_| crate::class_map::get_class_map());
}

#[test]
#[serial]
#[should_panic(expected = "class_map: a data root is already held")]
fn rejects_class_mutation_recursion_when_class_callback_holds_lock() {
    crate::class_map::with_class_map_mut(|_| crate::class_map::reset_class_map());
}

#[test]
#[serial]
#[should_panic(expected = "file_map: a data root is already held")]
fn rejects_file_recursion_when_file_callback_holds_lock() {
    crate::file_map::with_file_map(|_| crate::file_map::get_file_num_by_filename("recursive"));
}

#[test]
#[serial]
#[should_panic(expected = "canonical_map: a data root is already held")]
fn rejects_canonical_recursion_when_canonical_callback_holds_lock() {
    crate::file_map::with_canonical_map(|_| crate::file_map::canonical("recursive"));
}

#[test]
#[serial]
#[should_panic(expected = "file_routes: a data root is already held")]
fn rejects_routes_recursion_when_routes_callback_holds_lock() {
    crate::file_routes::with_file_routes(|_| crate::file_routes::get_file_routes());
}

#[test]
#[serial]
#[should_panic(expected = "file_routes: a data root is already held")]
fn rejects_routes_recursion_when_lazy_iterator_runs_under_lock() {
    let _route_count = crate::file_routes::route_count_for_files(std::iter::once_with(|| {
        let _file_routes = crate::file_routes::get_file_routes();
        "recursive"
    }));
}

#[test]
#[serial]
#[should_panic(expected = "prefix: a data root is already held")]
fn rejects_prefix_recursion_when_prefix_callback_holds_lock() {
    crate::with_prefix(|_| crate::get_prefix());
}

#[test]
#[serial]
#[should_panic(expected = "key_buf: a data root is already held")]
fn rejects_scratch_recursion_when_builder_holds_borrow() {
    crate::class_slot_for_key("recursive", |_| {
        crate::class_slot_for_key("recursive", |_| {});
    });
}

#[test]
#[serial]
fn releases_root_marker_when_callback_unwinds() {
    // Given
    crate::class_map::reset_class_map();
    // When
    let failed = std::panic::catch_unwind(|| {
        crate::class_map::with_class_map(|_| panic!("callback unwound"));
    });
    // Then
    assert!(failed.is_err());
    assert_eq!(
        crate::class_map::get_class_map(),
        std::collections::HashMap::new()
    );
}

#[test]
#[serial]
fn permits_distinct_root_and_sequential_reentry_when_owner_is_nested() {
    // Given
    crate::set_prefix(Some("nested-".into()));
    crate::debug::set_debug(false);
    // When
    let name = crate::admission::with_admission(|| {
        crate::admission::with_admission(|| crate::keyframes_name_of_escaped("frames"))
    });
    // Then
    assert_eq!(name, "nested-Kframes");
    crate::set_prefix(None);
}
