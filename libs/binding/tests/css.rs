use devup_ui_binding_core::{ExtractionMode, SourceMapMode};
use extractor::ExtractOption;

#[test]
fn numbered_css_selects_its_file_when_other_files_exist() -> Result<(), String> {
    // Given: two independently emitted files with explicit, nonconsecutive numbers.
    devup_ui_binding_core::reset_state_internal(|| {});
    devup_ui_binding_core::import_file_map_internal(
        [("red.tsx".to_string(), 7), ("blue.tsx".to_string(), 3)]
            .into_iter()
            .collect(),
    );
    let option = || ExtractOption {
        package: "@devup-ui/react".to_string(),
        css_dir: "df".to_string(),
        ..ExtractOption::default()
    };
    let mode = || ExtractionMode {
        source_map: SourceMapMode::Skip,
        resolver: None,
        import_main_css_in_css: false,
    };
    let red = devup_ui_binding_core::code_extract(
        (
            "red.tsx",
            "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"red\" />;",
        ),
        option(),
        mode(),
    )?;
    devup_ui_binding_core::code_extract(
        (
            "blue.tsx",
            "import { Box } from '@devup-ui/react'; export const view = <Box bg=\"blue\" />;",
        ),
        option(),
        mode(),
    )?;

    // When: the host requests the red file by its imported number.
    let selected = devup_ui_binding_core::get_css(Some(7), false);

    // Then: only the same file CSS returned by that transform is selected.
    assert_eq!(Some(selected), red.css);
    Ok(())
}
