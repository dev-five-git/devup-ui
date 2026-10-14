use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use bimap::BiHashMap;
use rstest::rstest;
use serial_test::serial;

use crate::{
    admission::with_admission,
    atom_hoist::{atom_plan, freeze_atom_plan, restore_atom_plan, set_atom_hoist},
    class_map::{get_class_map, set_class_map, with_class_map_mut},
    counter_allocation::reserve_counter,
    exact_attempt::with_exclusive_attempt,
    file_map::{
        get_file_map, get_file_num_by_filename, get_or_insert_original_id, get_original_ids,
        reset_canonical_map, set_file_map, set_original_ids,
    },
    file_routes::{reset_file_routes, set_file_routes},
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    classes: HashMap<String, HashMap<String, usize>>,
    files: BiHashMap<String, usize>,
    originals: BTreeMap<String, u32>,
    plan: Option<BTreeSet<String>>,
}

fn snapshot() -> State {
    State {
        classes: get_class_map(),
        files: get_file_map(),
        originals: get_original_ids(),
        plan: atom_plan(),
    }
}

fn fixture(seeded: bool, threshold: usize) -> State {
    let state = if seeded {
        State {
            classes: HashMap::from([
                ("old-empty".into(), HashMap::new()),
                ("seed".into(), HashMap::from([("kept".into(), 30)])),
            ]),
            files: BiHashMap::from_iter([("seed".into(), 42)]),
            originals: BTreeMap::from([("seed".into(), 32)]),
            plan: None,
        }
    } else {
        State {
            classes: HashMap::new(),
            files: BiHashMap::new(),
            originals: BTreeMap::new(),
            plan: None,
        }
    };
    set_class_map(state.classes.clone());
    set_file_map(state.files.clone());
    set_original_ids(state.originals.clone());
    restore_atom_plan(None);
    reset_canonical_map();
    set_file_routes(HashMap::from([("eligible".into(), HashSet::from([7]))]));
    set_atom_hoist(Some(threshold));
    state
}

fn register(stage: &str) {
    reserve_counter(stage, "temporary");
    with_class_map_mut(|classes| {
        classes.insert(format!("{stage}-empty"), HashMap::new());
    });
    let _ = get_file_num_by_filename(stage);
    get_or_insert_original_id(stage)
        .unwrap_or_else(|error| panic!("original registration for {stage} failed: {error:?}"));
}

fn expected_registration(before: &State, stage: &str) -> State {
    let mut expected = before.clone();
    expected
        .classes
        .insert(stage.into(), HashMap::from([("temporary".into(), 0)]));
    expected
        .classes
        .insert(format!("{stage}-empty"), HashMap::new());
    expected.files.insert(stage.into(), before.files.len());
    expected.originals.insert(
        stage.into(),
        u32::try_from(before.originals.len())
            .unwrap_or_else(|error| panic!("fixture original count exceeds u32: {error}")),
    );
    expected
}

fn expected_plan(threshold: usize) -> BTreeSet<String> {
    if threshold == 1 {
        BTreeSet::from(["eligible".into()])
    } else {
        BTreeSet::new()
    }
}

fn cleanup() {
    set_class_map(HashMap::new());
    set_file_map(BiHashMap::new());
    set_original_ids(BTreeMap::new());
    restore_atom_plan(None);
    set_atom_hoist(None);
    reset_file_routes();
}

#[rstest]
#[case(true, true, 1)]
#[case(true, false, 2)]
#[case(false, true, 2)]
#[case(false, false, 1)]
#[serial]
fn restores_all_maps_when_nested_error_precedes_outer_exit(
    #[case] outer_commits: bool,
    #[case] seeded: bool,
    #[case] threshold: usize,
) {
    // Given
    with_admission(|| {
        let before = fixture(seeded, threshold);
        let outer = expected_registration(&before, "outer");
        assert_eq!(snapshot(), before);
        // When
        let result = with_exclusive_attempt(|| {
            register("outer");
            assert_eq!(snapshot(), outer);
            let inner: Result<(), &str> = with_exclusive_attempt(|| {
                register("inner");
                freeze_atom_plan();
                let mut tentative = expected_registration(&outer, "inner");
                tentative.plan = Some(expected_plan(threshold));
                assert_eq!(snapshot(), tentative);
                set_class_map(HashMap::new());
                Err("inner")
            });
            assert_eq!(inner, Err("inner"));
            assert_eq!(snapshot(), outer);
            if outer_commits { Ok(()) } else { Err("outer") }
        });
        // Then
        assert_eq!(result, if outer_commits { Ok(()) } else { Err("outer") });
        assert_eq!(snapshot(), if outer_commits { outer } else { before });
        cleanup();
    });
}

#[rstest]
#[case(true, 1)]
#[case(false, 2)]
#[serial]
fn restores_all_maps_when_nested_unwind_is_caught_by_outer(
    #[case] outer_commits: bool,
    #[case] threshold: usize,
) {
    // Given
    with_admission(|| {
        let before = fixture(true, threshold);
        let outer = expected_registration(&before, "outer");
        assert_eq!(snapshot(), before);
        // When
        let result = with_exclusive_attempt(|| {
            register("outer");
            assert_eq!(snapshot(), outer);
            let panic = std::panic::catch_unwind(|| {
                with_exclusive_attempt::<(), ()>(|| {
                    register("inner");
                    freeze_atom_plan();
                    let mut tentative = expected_registration(&outer, "inner");
                    tentative.plan = Some(expected_plan(threshold));
                    assert_eq!(snapshot(), tentative);
                    panic!("nested build");
                })
            });
            let Err(payload) = panic else {
                panic!("nested build unexpectedly returned without panicking");
            };
            assert_eq!(payload.downcast_ref::<&str>(), Some(&"nested build"));
            assert_eq!(snapshot(), outer);
            if outer_commits { Ok(()) } else { Err(()) }
        });
        // Then
        assert_eq!(result, if outer_commits { Ok(()) } else { Err(()) });
        assert_eq!(snapshot(), if outer_commits { outer } else { before });
        cleanup();
    });
}

#[rstest]
#[case(1)]
#[case(2)]
#[serial]
fn restores_all_maps_when_outer_aborts_after_inner_commit(#[case] threshold: usize) {
    // Given
    with_admission(|| {
        let before = fixture(true, threshold);
        assert_eq!(snapshot(), before);
        // When
        let result: Result<(), ()> = with_exclusive_attempt(|| {
            register("outer");
            let outer = expected_registration(&before, "outer");
            assert_eq!(snapshot(), outer);
            with_exclusive_attempt::<(), ()>(|| {
                register("inner");
                freeze_atom_plan();
                Ok(())
            })?;
            let mut committed = expected_registration(&outer, "inner");
            committed.plan = Some(expected_plan(threshold));
            assert_eq!(snapshot(), committed);
            Err(())
        });
        // Then
        assert_eq!(result, Err(()));
        assert_eq!(snapshot(), before);
        cleanup();
    });
}

#[test]
#[serial]
fn restores_all_maps_when_nested_unwind_escapes_outer() {
    // Given
    with_admission(|| {
        let before = fixture(true, 1);
        assert_eq!(snapshot(), before);
        // When
        let panic = std::panic::catch_unwind(|| {
            with_exclusive_attempt::<(), ()>(|| {
                register("outer");
                with_exclusive_attempt::<(), ()>(|| {
                    register("inner");
                    freeze_atom_plan();
                    panic!("escaping nested build");
                })
            })
        });
        // Then
        let Err(payload) = panic else {
            panic!("escaping nested build unexpectedly returned without panicking");
        };
        assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"escaping nested build")
        );
        assert_eq!(snapshot(), before);
        cleanup();
    });
}
