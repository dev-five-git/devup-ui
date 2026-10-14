use super::*;

#[rstest]
#[case("")]
#[case("du-FLa-")]
#[serial]
fn cache_restores_original_counters_without_extra_slots_or_dangling_references(
    #[case] prefix: &str,
) {
    // Given
    setup();
    set_prefix(Some(prefix.into()));
    let sources = [
        (
            "a.tsx",
            "import {Box} from '@devup-ui/react';export const x=<Box color='red' p={4}/>;",
        ),
        (
            "child.tsx",
            "import {Box} from '@devup-ui/react';export const x=<Box color='blue' p={4}/>;",
        ),
    ];
    let before = sources.map(|(file, source)| compile(file, source).code());
    let sheet_json = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let classes = export_class_map_internal().unwrap_or_else(|error| panic!("{error}"));
    let files = css::file_map::get_file_map();
    let canonical_map = css::file_map::get_canonical_map();
    let ids = css::file_map::get_original_ids();
    let css_before = delivered();
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    set_file_map(files.clone());
    set_canonical_map(canonical_map.clone());
    set_class_map(serde_json::from_str(&classes).unwrap_or_else(|error| panic!("{error}")));
    import_sheet_internal(
        serde_json::from_str(&sheet_json).unwrap_or_else(|error| panic!("{error}")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // When
    let after: Vec<_> = sources
        .iter()
        .rev()
        .map(|(file, source)| compile(file, source).code())
        .collect();
    // Then
    assert_eq!(after, before.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(delivered(), css_before);
    assert_eq!(
        export_class_map_internal().unwrap_or_else(|error| panic!("{error}")),
        classes
    );
    assert_eq!(css::file_map::get_file_map(), files);
    assert_eq!(css::file_map::get_canonical_map(), canonical_map);
    assert_eq!(css::file_map::get_original_ids(), ids);
    with_style_sheet(|sheet| assert_eq!(sheet.names.len(), 0));
    reset_build_state_internal();
}
