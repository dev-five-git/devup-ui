use std::{collections::HashMap, sync::mpsc};

use serial_test::serial;

use crate::{
    admission::{observer::wait_for_thread, with_admission},
    allocation_input::NameMode,
    class_map::{
        get_class_map, reset_class_map, set_class_map, with_class_map, with_class_map_mut,
    },
    counter_names::allocate_name,
    counter_test_helpers::{context, declaration},
    exact_attempt::with_exclusive_attempt,
};

fn after_abort<R: Send + 'static>(contender: impl FnOnce() -> R + Send + 'static) -> R {
    let (sent, received) = mpsc::channel();
    let worker = with_admission(|| {
        let mut worker = None;
        let result: Result<(), ()> = with_exclusive_attempt(|| {
            let _owner_allocation =
                allocate_name(&declaration(Some("red")), &context(NameMode::Counter, None));
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
            Err(())
        });
        assert_eq!(result, Err(()));
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
fn public_read_waits_and_observes_snapshot_when_owner_aborts() {
    // Given
    let before = HashMap::from([("empty".into(), HashMap::new())]);
    set_class_map(before.clone());
    // When
    let observed = after_abort(get_class_map);
    // Then
    assert_eq!(observed, before);
    reset_class_map();
}

#[test]
#[serial]
fn public_callback_read_waits_when_owner_aborts() {
    // Given
    reset_class_map();
    // When
    let observed = after_abort(|| with_class_map(Clone::clone));
    // Then
    assert_eq!(observed, HashMap::new());
}

#[test]
#[serial]
fn public_allocation_survives_when_owner_aborts() {
    // Given
    reset_class_map();
    // When
    let name = after_abort(|| {
        allocate_name(
            &declaration(Some("blue")),
            &context(NameMode::Counter, None),
        )
    });
    // Then
    assert_eq!(name.name, "app-a");
    assert_eq!(
        get_class_map(),
        HashMap::from([(
            String::new(),
            HashMap::from([("color-0-blue--255".into(), 0)]),
        )])
    );
    reset_class_map();
}

#[test]
#[serial]
fn tentative_reuse_becomes_retained_allocation_when_owner_aborts() {
    // Given
    reset_class_map();
    // When
    let name =
        after_abort(|| allocate_name(&declaration(Some("red")), &context(NameMode::Counter, None)));
    // Then: bypass would reuse the owner's receipt and then lose this key.
    assert_eq!(name.name, "app-a");
    assert_eq!(
        get_class_map(),
        HashMap::from([(
            String::new(),
            HashMap::from([("color-0-red--255".into(), 0)]),
        )])
    );
    reset_class_map();
}

#[test]
#[serial]
fn arbitrary_mutation_survives_when_owner_aborts() {
    // Given
    reset_class_map();
    // When
    after_abort(|| {
        with_class_map_mut(|map| {
            map.insert("writer".into(), HashMap::from([("kept".into(), 47)]));
        });
    });
    // Then
    assert_eq!(
        get_class_map(),
        HashMap::from([("writer".into(), HashMap::from([("kept".into(), 47)]))])
    );
    reset_class_map();
}

#[test]
#[serial]
fn reset_runs_after_restore_when_owner_aborts() {
    // Given
    set_class_map(HashMap::from([("old-empty".into(), HashMap::new())]));
    // When
    after_abort(reset_class_map);
    // Then
    assert_eq!(get_class_map(), HashMap::new());
}

#[test]
#[serial]
fn replacement_runs_after_restore_when_owner_aborts() {
    // Given
    reset_class_map();
    let replacement = HashMap::from([("replacement-empty".into(), HashMap::new())]);
    let expected = replacement.clone();
    // When
    after_abort(move || set_class_map(replacement));
    // Then
    assert_eq!(get_class_map(), expected);
    reset_class_map();
}

#[test]
#[serial]
fn read_observes_committed_map_when_owner_succeeds() {
    // Given
    reset_class_map();
    let (sent, received) = mpsc::channel();
    // When
    let worker = with_admission(|| {
        let mut worker = None;
        let result = with_exclusive_attempt::<_, ()>(|| {
            let _owner_allocation =
                allocate_name(&declaration(Some("red")), &context(NameMode::Counter, None));
            worker = Some(std::thread::spawn(move || {
                sent.send(get_class_map())
                    .unwrap_or_else(|error| panic!("committed map send failed: {error}"));
            }));
            let Some(worker) = worker.as_ref() else {
                panic!("map reader worker was not spawned");
            };
            wait_for_thread(worker.thread().id());
            assert!(matches!(
                received.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            Ok(())
        });
        assert_eq!(result, Ok(()));
        worker.unwrap_or_else(|| panic!("map reader worker was not retained"))
    });
    let observed = received
        .recv()
        .unwrap_or_else(|error| panic!("committed map receive failed: {error}"));
    worker
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    // Then
    assert_eq!(
        observed,
        HashMap::from([(
            String::new(),
            HashMap::from([("color-0-red--255".into(), 0)]),
        )])
    );
    reset_class_map();
}
