use super::*;
use serial_test::serial;

fn seed() {
    crate::set_prefix(Some("outer".into()));
    debug::set_debug(true);
    atom_hoist::set_atom_hoist(Some(3));
    class_map::set_class_map(HashMap::from([(
        "x".into(),
        HashMap::from([("y".into(), 7)]),
    )]));
    let _file_num = file_map::get_file_num_by_filename("outer.tsx");
    file_map::set_canonical_map(HashMap::from([("member".into(), "root".into())]));
    file_routes::set_file_routes(HashMap::from([("outer.tsx".into(), HashSet::from([2]))]));
    crate::set_custom_shorthands(BTreeMap::from([("insetX".into(), vec!["left".into()])]));
    theme_tokens::set_theme_token_levels(
        BTreeMap::from([("space".into(), vec![0, 2])]),
        BTreeMap::from([("card".into(), vec![0, 3])]),
    );
    theme_tokens::set_typography_keys(vec!["body".into()]);
}

fn assert_seed() {
    assert_eq!(crate::get_prefix().as_deref(), Some("outer"));
    assert!(debug::is_debug());
    assert_eq!(atom_hoist::atom_hoist_threshold(), Some(3));
    assert_eq!(class_map::get_class_map()["x"]["y"], 7);
    assert_eq!(file_map::get_file_map().len(), 1);
    assert_eq!(file_map::get_canonical_map()["member"], "root");
    assert_eq!(
        file_routes::get_file_routes()["outer.tsx"],
        HashSet::from([2])
    );
    assert_eq!(
        crate::disassemble_property("insetX").collect::<Vec<_>>(),
        vec!["left"]
    );
    assert_eq!(
        theme_tokens::get_responsive_theme_token("$space"),
        Some(vec![0, 2])
    );
    assert_eq!(
        theme_tokens::get_responsive_theme_token("$card"),
        Some(vec![0, 3])
    );
    assert_eq!(theme_tokens::get_typography_keys(), vec!["body"]);
}

fn assert_clean() {
    assert_eq!(crate::get_prefix(), None);
    assert!(!debug::is_debug());
    assert_eq!(atom_hoist::atom_hoist_threshold(), None);
    assert_eq!(class_map::get_class_map(), HashMap::new());
    assert_eq!(file_map::get_file_map().len(), 0);
    assert_eq!(file_map::get_canonical_map(), HashMap::new());
    assert_eq!(file_routes::get_file_routes(), HashMap::new());
    assert_eq!(crate::get_custom_shorthand_names(), Vec::<String>::new());
    assert!(!crate::HAS_CUSTOM_SHORTHANDS.load(Ordering::Relaxed));
    assert_eq!(theme_tokens::get_responsive_theme_token("$space"), None);
    assert_eq!(theme_tokens::get_responsive_theme_token("$card"), None);
    assert_eq!(theme_tokens::get_typography_keys(), Vec::<String>::new());
}

#[test]
#[serial]
fn reset_clears_all_groups_when_populated() {
    let _state = TestStateGuard::default();
    seed();
    reset_state_for_testing();
    assert_clean();
}

#[test]
#[serial]
fn reset_is_idempotent_when_empty() {
    let _state = TestStateGuard::new();
    reset_state_for_testing();
    reset_state_for_testing();
    assert_clean();
}

#[test]
#[serial]
fn scope_restores_all_groups_when_nested() {
    let _state = TestStateGuard::new();
    seed();
    {
        let _nested = TestStateGuard::new();
        assert_clean();
        crate::set_prefix(Some("inner".into()));
    }
    assert_seed();
}

#[test]
#[serial]
fn scope_restores_all_groups_when_unwinding() {
    let _state = TestStateGuard::new();
    seed();
    let result = std::panic::catch_unwind(|| {
        let _nested = TestStateGuard::new();
        crate::set_prefix(Some("inner".into()));
        panic!("test unwind");
    });
    assert!(result.is_err());
    assert_seed();
}
