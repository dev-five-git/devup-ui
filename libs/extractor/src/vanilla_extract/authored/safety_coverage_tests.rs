use serial_test::serial;

use crate::{
    ExtractOption,
    vanilla_extract::{Stylesheet, execute_authored},
};

#[test]
#[serial]
fn compiler_input_is_rejected_when_a_fallback_uses_it_as_an_ordinary_value() -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let source = "import {css} from '@devup-ui/react';\nexport const help=()=>({css});";
    let stylesheet = Stylesheet {
        filename: "/compiler-value.css.ts",
        code: source,
        source,
        edits: &[],
    };
    // When
    let result = execute_authored(stylesheet, &ExtractOption::default(), None);
    // Then
    let error = result
        .err()
        .ok_or("compiler API value escaped fallback safety")?;
    assert!(error.starts_with("/compiler-value.css.ts:2:25:"), "{error}");
    assert!(error.contains("stylesheet export `help` cannot use `css` at build time: authored fallback references binding `css` removed by stylesheet compilation"), "{error}");
    assert!(error.contains("Fix: make the export self-contained or move its runtime dependency to an ordinary module"), "{error}");
    Ok(())
}
