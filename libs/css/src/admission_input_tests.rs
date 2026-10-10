use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::mpsc,
};

use serial_test::serial;

use crate::{
    admission::{observer::wait_for_thread, with_admission},
    allocation_input::{AllocationFile, capture_context},
    atom_hoist::{atom_plan, freeze_atom_plan, restore_atom_plan, set_atom_hoist},
    counter_test_helpers::declaration,
    exact_attempt::with_exclusive_attempt,
    file_map,
};

fn blocked_then<R: Send + 'static>(
    contender: impl FnOnce() -> R + Send + 'static,
    inspect: impl FnOnce(),
    inspect_restored: impl FnOnce(),
) -> R {
    let (sent, received) = mpsc::channel();
    let worker = with_admission(|| {
        let mut worker = None;
        let result: Result<(), ()> = with_exclusive_attempt(|| {
            worker = Some(std::thread::spawn(move || {
                sent.send(contender())
                    .unwrap_or_else(|error| panic!("contender send failed: {error}"));
            }));
            let Some(worker) = worker.as_ref() else {
                panic!("contender worker was not spawned");
            };
            wait_for_thread(worker.thread().id());
            assert!(matches!(
                received.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            inspect();
            Err(())
        });
        assert_eq!(result, Err(()));
        inspect_restored();
        worker.unwrap_or_else(|| panic!("contender worker was not retained"))
    });
    let output = received
        .recv()
        .unwrap_or_else(|error| panic!("contender receive failed: {error}"));
    worker
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    output
}

#[test]
#[serial]
fn configuration_setter_waits_when_exact_capture_owns_inputs() {
    // Given
    crate::set_prefix(Some("owner-".into()));
    crate::debug::set_debug(false);
    set_atom_hoist(None);
    // When
    blocked_then(
        || crate::set_prefix(Some("writer-".into())),
        || {
            let captured = capture_context(
                &declaration(Some("red")),
                None,
                crate::CounterOwner::Inactive,
            );
            assert_eq!(captured.config.prefix, "owner-");
        },
        || {},
    );
    // Then
    assert_eq!(crate::get_prefix(), Some("writer-".into()));
    crate::set_prefix(None);
}

#[test]
#[serial]
fn original_registration_waits_when_owner_restores_before_release() {
    // Given
    let before = BTreeMap::new();
    file_map::set_original_ids(before.clone());
    // When
    let registered = blocked_then(
        || file_map::get_or_insert_original_id("writer"),
        || {
            assert_eq!(file_map::get_or_insert_original_id("owner"), Ok(0));
            assert_eq!(file_map::original_id("writer"), None);
            assert_eq!(
                file_map::get_original_ids(),
                BTreeMap::from([("owner".into(), 0)])
            );
        },
        || {
            assert_eq!(file_map::get_original_ids(), before);
        },
    );
    // Then
    assert_eq!(registered, Ok(0));
    assert_eq!(
        file_map::get_original_ids(),
        BTreeMap::from([("writer".into(), 0)])
    );
    file_map::set_original_ids(BTreeMap::new());
}

#[test]
#[serial]
fn plan_reader_waits_when_owner_restores_before_release() {
    // Given
    let before = None;
    restore_atom_plan(before.clone());
    set_atom_hoist(Some(1));
    crate::file_routes::set_file_routes(HashMap::from([("tentative".into(), HashSet::from([7]))]));
    file_map::reset_canonical_map();
    // When
    let observed = blocked_then(
        atom_plan,
        || {
            freeze_atom_plan();
            assert_eq!(atom_plan(), Some(BTreeSet::from(["tentative".into()])));
        },
        || {
            assert_eq!(atom_plan(), before);
        },
    );
    // Then
    assert_eq!(observed, None);
    set_atom_hoist(None);
    crate::file_routes::reset_file_routes();
}

#[test]
#[serial]
fn seed_waits_and_registers_originals_and_canonical_delivery_when_owner_releases() {
    // Given
    file_map::reset_file_map();
    file_map::set_canonical_map(HashMap::from([("child".into(), "root".into())]));
    // When
    blocked_then(
        || file_map::seed_file_numbers(&["child".into(), "root".into()]),
        || {
            assert_eq!(file_map::get_original_ids(), BTreeMap::new());
            assert_eq!(file_map::get_file_map(), bimap::BiHashMap::new());
        },
        || {},
    );
    // Then
    assert_eq!(
        file_map::get_original_ids(),
        BTreeMap::from([("child".into(), 0), ("root".into(), 1)])
    );
    assert_eq!(
        file_map::get_file_map(),
        bimap::BiHashMap::from_iter([("root".into(), 0)])
    );
    file_map::reset_canonical_map();
    file_map::reset_file_map();
}

#[test]
#[serial]
fn reset_waits_and_clears_files_and_originals_when_owner_releases() {
    // Given
    file_map::reset_file_map();
    file_map::seed_file_numbers(&["owner".into()]);
    // When
    blocked_then(
        file_map::reset_file_map,
        || {
            assert_eq!(file_map::original_id("owner"), Some(0));
            assert_eq!(file_map::get_file_num_by_filename("owner"), 0);
        },
        || {},
    );
    // Then
    assert_eq!(file_map::get_original_ids(), BTreeMap::new());
    assert_eq!(file_map::get_file_map(), bimap::BiHashMap::new());
}

#[test]
#[serial]
fn canonical_setter_waits_and_updates_collapsed_state_when_owner_releases() {
    // Given
    file_map::reset_canonical_map();
    // When
    blocked_then(
        || file_map::set_canonical_map(HashMap::from([("child".into(), "root".into())])),
        || {
            assert_eq!(file_map::canonical("child"), "child");
            assert!(!crate::naming::is_shared_namespace(Some("root")));
        },
        || {},
    );
    // Then
    assert_eq!(file_map::canonical("child"), "root");
    assert!(crate::naming::is_shared_namespace(Some("root")));
    file_map::reset_canonical_map();
}

#[test]
#[serial]
fn capture_and_plan_allow_nested_distinct_roots_when_owner_reenters() {
    // Given
    crate::set_prefix(Some("captured-".into()));
    set_atom_hoist(Some(1));
    restore_atom_plan(None);
    crate::file_routes::set_file_routes(HashMap::from([("child".into(), HashSet::from([7]))]));
    file_map::set_canonical_map(HashMap::from([("child".into(), "root".into())]));
    // When
    let captured = with_exclusive_attempt::<_, ()>(|| {
        freeze_atom_plan();
        Ok(capture_context(
            &declaration(Some("red")),
            Some("child"),
            crate::CounterOwner::D9(42),
        ))
    })
    .unwrap_or_else(|()| panic!("nested capture attempt unexpectedly failed"));
    // Then
    assert_eq!(captured.file, Some(AllocationFile::Original(42)));
    assert_eq!(captured.config.prefix, "captured-");
    assert_eq!(
        captured.delivery,
        Some(crate::allocation_input::CapturedDelivery {
            canonical: "root".into(),
            hoisted: true
        })
    );
    assert_eq!(atom_plan(), Some(BTreeSet::from(["root".into()])));
    set_atom_hoist(None);
    restore_atom_plan(None);
    crate::file_routes::reset_file_routes();
    file_map::reset_canonical_map();
    crate::set_prefix(None);
}

#[test]
#[serial]
fn delivery_registration_waits_when_owner_restores_before_release() {
    // Given
    let before = bimap::BiHashMap::from_iter([("seed".into(), 42)]);
    let originals = BTreeMap::from([("seed".into(), 32)]);
    file_map::set_file_map(before.clone());
    file_map::set_original_ids(originals.clone());
    // When
    let registered = blocked_then(
        || file_map::get_file_num_by_filename("writer"),
        || {
            assert_eq!(file_map::get_file_num_by_filename("owner"), 1);
            assert_eq!(
                file_map::get_file_map(),
                bimap::BiHashMap::from_iter([("seed".into(), 42), ("owner".into(), 1)])
            );
        },
        || {
            assert_eq!(file_map::get_file_map(), before);
            assert_eq!(file_map::get_original_ids(), originals);
        },
    );
    // Then
    assert_eq!(registered, 1);
    assert_eq!(
        file_map::get_file_map(),
        bimap::BiHashMap::from_iter([("seed".into(), 42), ("writer".into(), 1)])
    );
    assert_eq!(file_map::get_original_ids(), originals);
    file_map::reset_file_map();
}
