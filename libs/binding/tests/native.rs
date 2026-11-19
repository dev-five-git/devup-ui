use devup_ui_binding_core::code_extract_native;

const SOURCE: &str =
    "import { Box } from '@devup-ui/react'\nexport const view = <Box bg=\"red\" p={1} />\n";

#[test]
fn native_extraction_gives_code_and_a_map_without_css() -> Result<(), String> {
    // When
    let output = code_extract_native(("view.tsx", SOURCE), "@devup-ui/react", "android", true)?;
    // Then: React Native's View with a hoisted style, and nothing for a stylesheet.
    assert!(output.code.contains("View as Box"));
    assert!(output.code.contains("backgroundColor: \"red\""));
    assert!(output.code.contains("padding: 4"));
    assert!(output.map.is_some());
    assert!(output.css.is_none());
    assert!(output.css_file.is_none());
    assert!(!output.updated_base_style);
    assert_eq!(output.dependencies, Vec::<String>::new());
    Ok(())
}

#[test]
fn native_extraction_rejects_an_unknown_platform() {
    // When
    let error = code_extract_native(("view.tsx", SOURCE), "@devup-ui/react", "windows", true)
        .err()
        .unwrap_or_default();
    // Then
    assert_eq!(
        error,
        "unknown React Native platform `windows`: use `ios`, `android` or `web`"
    );
}

#[test]
fn native_extraction_reports_build_errors() {
    // When
    let error = code_extract_native(
        (
            "view.tsx",
            "import { Box } from '@devup-ui/react'\n<Box p={[1, 2]} />\n",
        ),
        "@devup-ui/react",
        "ios",
        false,
    )
    .err()
    .unwrap_or_default();
    // Then
    assert!(error.starts_with("view.tsx:2:6: `<Box>` cannot use `p` on React Native"));
    assert!(error.contains("`runtime: false`"));
}
