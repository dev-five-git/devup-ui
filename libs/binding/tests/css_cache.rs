use std::collections::{HashMap, HashSet};

use devup_ui_binding_core::{
    ExtractionMode, GLOBAL_STYLE_SHEET, Output, SourceMapMode, code_extract, get_css,
    import_canonical_map_internal, import_file_map_internal, import_file_routes_internal,
    reset_state_internal, set_atom_hoist, with_style_sheet,
};
use extractor::ExtractOption;
use serial_test::serial;

fn extract(filename: &str, code: &str, single_css: bool) -> Result<Output, String> {
    code_extract(
        (filename, code),
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "df".to_string(),
            single_css,
            ..ExtractOption::default()
        },
        ExtractionMode {
            source_map: SourceMapMode::Skip,
            resolver: None,
            resolver_check: None,
            import_main_css_in_css: false,
        },
    )
}

#[test]
#[serial]
fn css_is_reused_until_an_extraction_changes_the_sheet() -> Result<(), String> {
    // Given: a transform that added a style, and the base read once.
    reset_state_internal(|| {});
    let red = extract(
        "red.tsx",
        "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"red\" />;",
        true,
    )?;
    let first = get_css(None, false);

    // When: the base is read again, then another transform adds a style.
    let again = get_css(None, true);
    extract(
        "blue.tsx",
        "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"blue\" />;",
        true,
    )?;
    let changed = get_css(None, false);

    // Then: the unchanged sheet reads the same, the changed one holds both.
    assert_eq!(red.css.as_deref(), Some(first.as_str()));
    assert_eq!(again, first);
    assert!(!first.contains("blue"));
    assert!(changed.contains("red") && changed.contains("blue"));
    Ok(())
}

#[test]
#[serial]
fn css_survives_a_source_extracted_again_unchanged() -> Result<(), String> {
    // Given: a source extracted once, with a global rule, and its base read.
    reset_state_internal(|| {});
    let source = "import { Box, globalCss } from '@devup-ui/react'; globalCss({ body: { m: 0 } }); export const view = <Box bg=\"red\" />;";
    extract("page.tsx", source, true)?;
    let first = get_css(None, false);

    // When: the same source is extracted again, as a prewarm and its transform do.
    let again = extract("page.tsx", source, true)?;

    // Then: the base reads the same as from a fresh serialization.
    assert_eq!(get_css(None, false), first);
    assert!(again.updated_base_style);
    assert!(first.contains("red"));
    Ok(())
}

#[test]
#[serial]
fn css_is_forgotten_when_the_sheet_is_borrowed_mutably() {
    // Given: the base of an empty sheet, read once.
    reset_state_internal(|| {});
    let empty = get_css(None, false);

    // When: a host test seam edits the locked sheet directly.
    GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .add_import("page.tsx", "theme.css");

    // Then: the next read serializes the edited sheet.
    let edited = get_css(None, false);
    assert_ne!(edited, empty);
    assert!(edited.contains("@import \"theme.css\";"));
}

/// The file stylesheet and the base, as a fresh serialization writes them now.
fn fresh() -> (String, String) {
    with_style_sheet(|sheet| {
        (
            sheet.create_css(Some("shared.tsx"), false),
            sheet.create_css(None, false),
        )
    })
}

/// The file stylesheet and the base, as the cached reads return them now.
fn read() -> (String, String) {
    (get_css(Some(0), false), get_css(None, false))
}

#[test]
#[serial]
fn css_follows_every_atom_hoisting_setting() -> Result<(), String> {
    // Given: a source extracted before atom hoisting, read once.
    reset_state_internal(|| {});
    import_file_map_internal(std::iter::once(("shared.tsx".to_string(), 0)).collect());
    extract(
        "shared.tsx",
        "import { Box } from '@devup-ui/react'; export const view = <Box styleOrder={1} color=\"red\" />;",
        false,
    )?;
    assert_eq!(read(), fresh());

    // When: each setting the serialization reads changes after a read.
    import_file_routes_internal(HashMap::from([(
        "shared.tsx".to_string(),
        HashSet::from([0, 1]),
    )]));
    let routed = (read(), fresh());
    import_canonical_map_internal(HashMap::new());
    let canonical = (read(), fresh());
    set_atom_hoist(Some(2));
    let hoisted = (read(), fresh());
    set_atom_hoist(None);
    let plain = (read(), fresh());

    // Then: every read is what a fresh serialization writes at that point.
    assert_eq!(routed.0, routed.1);
    assert_eq!(canonical.0, canonical.1);
    assert_eq!(hoisted.0, hoisted.1);
    assert_eq!(plain.0, plain.1);
    reset_state_internal(|| {});
    Ok(())
}

#[test]
#[serial]
fn css_is_forgotten_when_a_commit_freezes_the_atom_plan() -> Result<(), String> {
    // Given: atom hoisting over two routes, the empty sheet read once.
    reset_state_internal(|| {});
    import_file_map_internal(std::iter::once(("shared.tsx".to_string(), 0)).collect());
    import_file_routes_internal(HashMap::from([(
        "shared.tsx".to_string(),
        HashSet::from([0, 1]),
    )]));
    set_atom_hoist(Some(2));
    read();

    // When: a source without styles commits, freezing the plan into the sheet.
    let output = extract(
        "shared.tsx",
        "import { Box } from '@devup-ui/react'; export const view = 1;",
        false,
    )?;

    // Then: nothing was collected, yet the read follows the frozen plan.
    assert!(output.css.is_none());
    assert!(with_style_sheet(|sheet| sheet.atom_plan.is_some()));
    assert_eq!(read(), fresh());
    reset_state_internal(|| {});
    Ok(())
}
