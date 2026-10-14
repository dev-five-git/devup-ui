use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p", "padding", "4px")]
#[case("w", "width", "4px")]
#[case("transitionDuration", "transition-duration", "1ms")]
#[serial]
fn sheet_uses_extracted_numeric_declaration_when_type_proves_a_number(
    #[case] prop: &str,
    #[case] property: &str,
    #[case] scale: &str,
    #[values(false, true)] single_css: bool,
) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let filename = "scaled.tsx";
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(n:number){{return <Box {prop}={{n}}/>}}"
    );
    let output = extractor::extract(
        filename,
        &source,
        extractor::ExtractOption {
            single_css,
            ..extractor::ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let mut sheet = StyleSheet::default();
    sheet
        .update_styles(&output.styles, filename, single_css)
        .unwrap_or_else(|error| panic!("{error}"));
    let emitted = sheet.create_css((!single_css).then_some(filename), false);
    let dynamic = output
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture must have a dynamic declaration"));
    assert!(
        emitted.contains(&format!(
            "{property}:calc(var({}) * {scale})",
            dynamic.variable_name()
        )),
        "{emitted}"
    );
    assert!(
        emitted.contains(&format!("{}:initial", dynamic.variable_name())),
        "{emitted}"
    );
}
