use std::collections::HashMap;

#[path = "exact_attempt_nesting_tests.rs"]
mod nesting;

use serial_test::serial;

use crate::{
    allocation_input::NameMode,
    class_map::{Attempt, get_class_map, reset_class_map, set_class_map, with_class_map_mut},
    counter_allocation::reserve_counter,
    counter_names::allocate_name,
    counter_test_helpers::{context, declaration, variable},
    exact_attempt::with_exclusive_attempt,
};

fn seeded() -> HashMap<String, HashMap<String, usize>> {
    HashMap::from([
        ("old-empty".into(), HashMap::new()),
        ("seed".into(), HashMap::from([("kept".into(), 30)])),
    ])
}

#[test]
#[serial]
fn restores_exact_namespaces_and_slots_when_build_errors() {
    // Given
    let before = seeded();
    set_class_map(before.clone());
    // When
    let result: Result<(), &str> = with_exclusive_attempt(|| {
        assert_eq!(reserve_counter("seed", "kept").index(), 30);
        reserve_counter("old-empty", "temporary");
        reserve_counter("new", "temporary");
        with_class_map_mut(|map| {
            map.insert("new-empty".into(), HashMap::new());
            map.get_mut("seed")
                .unwrap_or_else(|| panic!("seed namespace missing before slot replacement"))
                .insert("kept".into(), 90);
        });
        Err("output error")
    });
    // Then
    assert_eq!(result, Err("output error"));
    assert_eq!(get_class_map(), before);
    crate::admission::with_admission(|| {
        crate::admission::assert_administration_allowed("after error");
    });
    reset_class_map();
}

#[test]
#[serial]
fn restores_exact_map_when_owner_resets_it() {
    // Given
    let before = seeded();
    set_class_map(before.clone());
    // When
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        reserve_counter("seed", "temporary");
        reset_class_map();
        Err(())
    });
    // Then
    assert_eq!(result, Err(()));
    assert_eq!(get_class_map(), before);
    reset_class_map();
}

#[test]
#[serial]
fn restores_snapshot_after_journal_drop_when_owner_replaces_it() {
    // Given: the replacement collides with an outer receipt and a retained key.
    let before = seeded();
    set_class_map(before.clone());
    // When
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        with_class_map_mut(|map| {
            map.get_mut("seed")
                .unwrap_or_else(|| panic!("seed namespace missing before key removal"))
                .remove("kept");
        });
        reserve_counter("seed", "kept");
        set_class_map(HashMap::from([(
            "seed".into(),
            HashMap::from([("kept".into(), 73)]),
        )]));
        Err(())
    });
    // Then
    assert_eq!(result, Err(()));
    assert_eq!(get_class_map(), before);
    reset_class_map();
}

#[test]
#[serial]
fn reverses_both_no_site_requests_when_inner_commits_and_outer_aborts() {
    // Given
    let before = seeded();
    set_class_map(before.clone());
    let captured = context(NameMode::Counter, None);
    // When
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        let inner = Attempt::begin();
        let class = allocate_name(&declaration(Some("var(--app-b)")), &captured);
        let variable = allocate_name(&variable(), &captured);
        assert_eq!(
            (class.name.as_str(), variable.name.as_str()),
            ("app-a", "--app-b")
        );
        inner.commit();
        Err(())
    });
    // Then
    assert_eq!(result, Err(()));
    assert_eq!(get_class_map(), before);
    reset_class_map();
}

#[test]
#[serial]
fn restores_exact_map_when_prepared_work_unwinds() {
    // Given
    let before = seeded();
    set_class_map(before.clone());
    // When
    let result = std::panic::catch_unwind(|| {
        with_exclusive_attempt::<(), ()>(|| {
            let _prepared = Attempt::begin();
            reserve_counter("new", "prepared");
            with_class_map_mut(|map| {
                map.clear();
            });
            panic!("prepared output panicked")
        })
    });
    // Then
    assert!(result.is_err());
    assert_eq!(get_class_map(), before);
    crate::admission::with_admission(|| {
        crate::admission::assert_administration_allowed("after unwind");
    });
    reset_class_map();
}

#[test]
#[serial]
fn commits_only_successful_retry_when_prepared_attempt_drops() {
    // Given
    set_class_map(seeded());
    // When
    let result = with_exclusive_attempt::<_, ()>(|| {
        {
            let _prepared = Attempt::begin();
            reserve_counter("retry", "discarded");
        }
        with_exclusive_attempt::<_, ()>(|| Ok(reserve_counter("retry", "kept").index()))
    });
    // Then
    assert_eq!(result, Ok(0));
    let mut expected = seeded();
    expected.insert("retry".into(), HashMap::from([("kept".into(), 0)]));
    assert_eq!(get_class_map(), expected);
    reset_class_map();
}

#[test]
#[serial]
fn reverses_nested_exact_commit_when_outer_errors() {
    // Given
    let before = seeded();
    set_class_map(before.clone());
    // When
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        with_exclusive_attempt::<_, ()>(|| Ok(reserve_counter("inner", "committed")))?;
        Err(())
    });
    // Then
    assert_eq!(result, Err(()));
    assert_eq!(get_class_map(), before);
    reset_class_map();
}

#[test]
#[serial]
fn retains_outer_work_when_nested_exact_attempt_errors() {
    // Given
    set_class_map(seeded());
    // When
    let result = with_exclusive_attempt::<_, ()>(|| {
        reserve_counter("outer", "retained");
        let inner: Result<(), ()> = with_exclusive_attempt(|| {
            reset_class_map();
            Err(())
        });
        assert_eq!(inner, Err(()));
        Ok(reserve_counter("outer", "next").index())
    });
    // Then
    assert_eq!(result, Ok(1));
    let mut expected = seeded();
    expected.insert(
        "outer".into(),
        HashMap::from([("retained".into(), 0), ("next".into(), 1)]),
    );
    assert_eq!(get_class_map(), expected);
    reset_class_map();
}
