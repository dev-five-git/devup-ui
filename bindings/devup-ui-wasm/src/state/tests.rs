use crate::state::*;
use crate::*;
use serial_test::serial;
use std::collections::{BTreeMap, HashMap};

fn extract_fixture() -> Output {
    code_extract_internal(
        "fresh.tsx",
        "import { Box } from '@devup-ui/react'; export const x = <Box bg='red'/>;",
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap()
}

#[test]
#[serial]
fn reset_returns_fresh_extraction_when_engine_is_dirty() {
    let _state = TestStateGuard::new();
    let baseline = extract_fixture().code();
    let baseline_css = get_css(None, false).unwrap();
    set_prefix(Some("dirty".into()));
    set_debug(true);
    set_atom_hoist(Some(2));
    register_shorthands_internal(BTreeMap::from([("alias".into(), vec!["left".into()])]));
    register_theme_internal(
        serde_json::from_str(
            r#"{
        "colors":{"default":{"brand":"red"}},
        "length":{"default":{"space":["1px",null,"2px"]}},
        "shadow":{"default":{"card":["0 1px 2px black",null,"0 2px 4px black"]}},
        "typography":{"body":{"fontSize":"12px"}},
        "breakpoints":[0,900]
    }"#,
        )
        .unwrap(),
    );
    css::file_map::set_canonical_map(HashMap::from([("member".into(), "root".into())]));
    import_file_routes_internal(HashMap::from([(
        "fresh.tsx".into(),
        std::collections::HashSet::from([1, 2]),
    )]));
    extract_fixture();
    reset_state_for_testing();
    assert_eq!(
        export_sheet_internal().unwrap(),
        serde_json::to_string(&sheet::StyleSheet::default()).unwrap()
    );
    assert_eq!(get_default_theme().unwrap(), None);
    assert!(MODULE_RESOLVER.with_borrow(Option::is_none));
    assert_eq!(get_prefix(), None);
    assert!(!is_debug());
    assert_eq!(css::atom_hoist::atom_hoist_threshold(), None);
    assert_eq!(css::class_map::get_class_map(), HashMap::new());
    assert_eq!(css::file_map::get_file_map().len(), 0);
    assert_eq!(css::file_map::get_canonical_map(), HashMap::new());
    assert_eq!(css::file_routes::get_file_routes(), HashMap::new());
    assert_eq!(css::get_custom_shorthand_names(), Vec::<String>::new());
    assert_eq!(
        css::theme_tokens::get_responsive_theme_token("$space"),
        None
    );
    assert_eq!(css::theme_tokens::get_responsive_theme_token("$card"), None);
    assert_eq!(
        css::theme_tokens::get_typography_keys(),
        Vec::<String>::new()
    );
    assert_eq!(extract_fixture().code(), baseline);
    assert_eq!(get_css(None, false).unwrap(), baseline_css);
}

#[test]
#[serial]
fn reset_is_idempotent_when_engine_is_empty() {
    let _state = TestStateGuard::new();
    reset_state_internal();
    let baseline = export_sheet_internal().unwrap();
    reset_state_internal();
    assert_eq!(export_sheet_internal().unwrap(), baseline);
}

#[test]
#[serial]
fn binding_scope_restores_sheet_when_nested() {
    let _state = TestStateGuard::new();
    extract_fixture();
    let baseline = export_sheet_internal().unwrap();
    {
        let _nested = TestStateGuard::new();
        assert_eq!(
            get_css(None, false).unwrap(),
            sheet::StyleSheet::default().create_css(None, false)
        );
        set_prefix(Some("inner".into()));
        extract_fixture();
    }
    assert_eq!(export_sheet_internal().unwrap(), baseline);
    assert_eq!(get_prefix(), None);
}

#[test]
#[serial]
fn binding_scope_restores_sheet_when_unwinding() {
    let _state = TestStateGuard::new();
    extract_fixture();
    let baseline = export_sheet_internal().unwrap();
    let result = std::panic::catch_unwind(|| {
        let _nested = TestStateGuard::new();
        set_debug(true);
        extract_fixture();
        panic!("test unwind");
    });
    assert!(result.is_err());
    assert_eq!(export_sheet_internal().unwrap(), baseline);
    assert!(!is_debug());
}
