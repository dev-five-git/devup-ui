use devup_ui_binding_core::{
    ExtractionMode, SourceMapMode, code_extract, export_canonical_map_internal,
    export_class_map_internal, get_css, import_canonical_map_internal, remove_file_internal,
    reset_state_internal,
};
use extractor::ExtractOption;

fn extract(filename: &str, code: &str) -> Result<(), String> {
    code_extract(
        (filename, code),
        ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "df".to_string(),
            ..ExtractOption::default()
        },
        ExtractionMode {
            source_map: SourceMapMode::Skip,
            resolver: None,
            resolver_check: None,
            import_main_css_in_css: false,
        },
    )
    .map(drop)
}

fn file_num(filename: &str) -> usize {
    css::file_map::get_file_num_by_filename(filename)
}

#[test]
fn removed_files_leave_no_styles_of_their_own() -> Result<(), String> {
    // Given: a file with its own bucket and global CSS, beside another file.
    reset_state_internal(|| {});
    extract(
        "a.tsx",
        "import { Box, globalCss } from '@devup-ui/react'; globalCss({ body: { margin: 0 } }); export const view = <Box bg=\"red\" />;",
    )?;
    extract(
        "b.tsx",
        "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"blue\" />;",
    )?;
    assert!(get_css(None, false).contains("margin:0"));
    assert!(get_css(Some(file_num("a.tsx")), false).contains("red"));

    // When: the first file is deleted.
    assert!(remove_file_internal("a.tsx"));

    // Then: its global CSS, bucket and class names are gone; the other file stays.
    assert!(!get_css(None, false).contains("margin:0"));
    assert!(!get_css(Some(file_num("a.tsx")), false).contains("red"));
    assert!(get_css(Some(file_num("b.tsx")), false).contains("blue"));
    assert!(!export_class_map_internal()?.contains("a.tsx"));
    assert!(!remove_file_internal("a.tsx"));

    // Given: a file collapsed into the remaining file's bucket.
    import_canonical_map_internal([("c.tsx".to_string(), "b.tsx".to_string())].into());
    extract(
        "c.tsx",
        "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"green\" />;",
    )?;

    // When: the bucket's root goes first, the collapsed file still shares it.
    assert!(!remove_file_internal("b.tsx"));
    assert!(get_css(Some(file_num("b.tsx")), false).contains("green"));

    // When: the collapsed file goes too, it leaves the bucket.
    assert!(!remove_file_internal("c.tsx"));
    assert_eq!(export_canonical_map_internal()?, "{}");

    // Then: the bucket belongs to no other file and goes with its root.
    assert!(remove_file_internal("b.tsx"));
    assert!(!get_css(Some(file_num("b.tsx")), false).contains("green"));
    // Names in the shared scope stay: other files may hand out the same ones.
    assert!(!export_class_map_internal()?.contains("b.tsx"));
    Ok(())
}
