use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, extract};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(concat!("import {", "value", "} from '@devup-ui/react-values';"), "value")]
#[case("const data=require('@devup-ui/react-values');", "data.value")]
#[case("const data=import('@devup-ui/react-values');", "data.value")]
#[serial]
fn padding_owns_first_slot_when_data_shares_package_prefix(
    #[case] declaration: &str,
    #[case] value: &str,
) {
    // Given: a data module shares the compiler package prefix.
    let file = "/src/prefix.tsx";
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_map::seed_file_numbers(&[file.to_string()]);
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    let source = format!(
        "import {{Box}} from '@devup-ui/react';{declaration}export const View=<Box color={{{value}}} p={{4}}/>;"
    );
    // When: the native public extractor processes the JSX.
    let output =
        extract(file, &source, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
    // Then: imported data consumes zero slots; padding is the first own atom.
    assert!(output.code.contains("a-a"), "{}", output.code);
    assert!(!output.code.contains("a-b"), "{}", output.code);
    assert!(
        output.code.contains("RL") || output.code.contains("RH"),
        "{}",
        output.code
    );
    assert!(
        !output.code.contains("OL") && !output.code.contains("OH"),
        "{}",
        output.code
    );
    assert!(output.styles.iter().any(|style| {
        matches!(style, ExtractStyleValue::Static(padding)
            if padding.property == "padding" && padding.value == "16px" && padding.naming == css::Naming::Own)
    }));
}
