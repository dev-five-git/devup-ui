use super::*;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashSet;

fn configure() {
    set_debug(false);
    set_prefix(None);
    set_atom_hoist(Some(2));
    import_canonical_map_internal(HashMap::new());
    import_file_routes_internal(HashMap::new());
    register_theme_internal(serde_json::from_str("{}").unwrap_or_else(|error| panic!("{error}")));
    register_shorthands_internal(BTreeMap::from([("edge".into(), vec!["marginLeft".into()])]));
}

pub(super) fn warm_cache() -> (usize, usize) {
    let resolver: Box<ModuleResolver> = Box::new(|_, _| {
        Some(ModuleResolution::Resolved(ResolvedModule {
            path: "styles.css.ts".into(),
            code: "import {style} from '@devup-ui/react'; export const card=style({color:'red'});"
                .into(),
            source_type: None,
        }))
    });
    code_extract_with_modules_internal(
        "app.css.ts", "import {style} from '@devup-ui/react'; import {card} from './styles.css'; export const x=style([card,{background:'blue'}]);",
        "@devup-ui/react", "df".into(), false, false, false, HashMap::new(), None, &*resolver,
    ).unwrap_or_else(|error| panic!("{error}"));
    let sizes = extractor::test_state::cache_sizes_for_testing();
    assert!(sizes.0 > 0);
    assert!(sizes.1 > 0);
    sizes
}

#[test]
#[serial]
fn identical_normalized_replay_preserves_warm_cache_and_frozen_plan() {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    configure();
    let sizes = warm_cache();
    let plan = css::atom_hoist::atom_plan();
    // When
    configure();
    register_shorthands_internal(BTreeMap::from([(
        "edge".into(),
        vec!["margin-left".into()],
    )]));
    // Then
    assert_eq!(extractor::test_state::cache_sizes_for_testing(), sizes);
    assert_eq!(css::atom_hoist::atom_plan(), plan);
}

#[rstest]
#[case::debug(|| set_debug(true))]
#[case::prefix(|| set_prefix(Some("new".into())))]
#[case::threshold(|| set_atom_hoist(Some(3)))]
#[case::canonical(|| import_canonical_map_internal(HashMap::from([("a".into(), "b".into())])))]
#[case::routes(|| import_file_routes_internal(HashMap::from([("a".into(), HashSet::from([1]))])))]
#[case::theme(|| register_theme_internal(serde_json::from_str(r#"{"length":{"default":{"gap":8}}}"#).unwrap_or_else(|error| panic!("{error}"))))]
#[case::shorthand(|| register_shorthands_internal(BTreeMap::new()))]
#[case::shorthand_targets(|| register_shorthands_internal(BTreeMap::from([("edge".into(), vec!["marginRight".into()])])))]
#[serial]
fn semantic_change_invalidates_evaluation_but_keeps_stripped_sources(#[case] change: fn()) {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    configure();
    let sizes = warm_cache();
    // When
    change();
    // Then
    assert_eq!(
        extractor::test_state::cache_sizes_for_testing(),
        (0, sizes.1)
    );
}

#[rstest]
#[case::takeover(reset_build_state_internal)]
#[case::sheet(|| import_sheet_json_internal("{}").unwrap_or_else(|error| panic!("{error}")))]
#[case::classes(|| import_class_map_json_internal("{}").unwrap_or_else(|error| panic!("{error}")))]
#[case::files(|| import_file_map_json_internal("{}").unwrap_or_else(|error| panic!("{error}")))]
#[case::canonical(|| import_canonical_map_json_internal("{}").unwrap_or_else(|error| panic!("{error}")))]
#[case::reservation(|| {
    let saved = crate::reservation::export_reservation_state_internal().unwrap_or_else(|error| panic!("{error}"));
    import_reservation_state_json_internal(&saved).unwrap_or_else(|error| panic!("{error}"));
})]
#[serial]
fn bulk_replacement_invalidates_evaluation(#[case] replace: fn()) {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    configure();
    let sizes = warm_cache();
    // When
    replace();
    // Then
    assert_eq!(
        extractor::test_state::cache_sizes_for_testing(),
        (0, sizes.1)
    );
}
