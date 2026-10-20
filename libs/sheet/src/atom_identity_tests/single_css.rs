use super::*;

#[test]
#[serial]
fn direct_single_css_extraction_and_sheet_update_ignore_requested_atom_mode() {
    // Given
    let build = |atom: bool, debug| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        css::file_map::reset_canonical_map();
        css::atom_hoist::restore_atom_plan(None);
        css::atom_hoist::set_atom_hoist(atom.then_some(2));
        css::debug::set_debug(debug);
        css::set_prefix(None);
        let mut sheet = StyleSheet::default();
        let source = "import {Box,keyframes} from '@devup-ui/react'; const fade=keyframes({from:{opacity:0},to:{opacity:1}}); export const x=<Box color={tone} animation={`${fade} 1s`}/>;";
        // When
        let output = extractor::extract(
            "single.tsx",
            source,
            extractor::ExtractOption {
                single_css: true,
                ..extractor::ExtractOption::default()
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
        sheet.update_styles(&output.styles, "single.tsx", true);
        // Then
        assert_eq!(sheet.atom_plan, None);
        assert_eq!(css::atom_hoist::atom_plan(), None);
        (output.code, sheet.create_css(None, false))
    };
    for debug in [false, true] {
        assert_eq!(build(false, debug), build(true, debug));
    }
    css::atom_hoist::set_atom_hoist(None);
    css::debug::set_debug(false);
}

#[test]
#[serial]
fn suppressed_sheet_does_not_capture_an_existing_atom_plan() {
    // Given
    css::atom_hoist::set_atom_hoist(Some(2));
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["shared".into()])));
    let mut sheet = StyleSheet::default();
    let source = "import {Box} from '@devup-ui/react'; export const x=<Box color='red'/>;";
    // When
    let output = extractor::extract(
        "single.tsx",
        source,
        extractor::ExtractOption {
            single_css: true,
            ..extractor::ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    sheet.update_styles(&output.styles, "single.tsx", true);
    // Then
    assert_eq!(sheet.atom_plan, None);
    assert_eq!(
        css::atom_hoist::atom_plan(),
        Some(BTreeSet::from(["shared".into()]))
    );
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
}
