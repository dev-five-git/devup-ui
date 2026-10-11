use rstest::rstest;
use serial_test::serial;

use super::Unit;
use crate::{ExtractOption, ExtractOutput};

#[test]
fn retained_code_rejects_malformed_source_map_instead_of_inventing_a_trace() -> Result<(), String> {
    // Given
    let output = ExtractOutput {
        code: "const width = 4;".to_string(),
        styles: Default::default(),
        map: Some("{invalid".to_string()),
        css_file: None,
        dependencies: Vec::new(),
    };
    // When
    let error = Unit::retained("retained.ts", &output, "const width: number = 4;")
        .err()
        .ok_or("malformed map accepted")?;
    // Then
    assert!(error.starts_with("retained.ts:1:1: Cannot read stylesheet source map:"));
    Ok(())
}

#[test]
fn retained_code_uses_written_trace_when_extraction_leaves_the_source_unchanged()
-> Result<(), String> {
    // Given
    let code = "const width: number = 4;";
    let output = ExtractOutput {
        code: code.to_string(),
        styles: Default::default(),
        map: None,
        css_file: None,
        dependencies: Vec::new(),
    };
    // When
    let unit = Unit::retained("retained.ts", &output, code)?;
    // Then
    assert_eq!(
        unit.place(unit.script().find('4').ok_or("missing initializer")?),
        "retained.ts:1:23"
    );
    Ok(())
}

#[rstest]
#[case(None)]
#[case(Some(r#"{"version":3,"sources":[],"names":[],"mappings":""}"#))]
fn retained_code_rejects_missing_original_source_instead_of_inventing_a_trace(
    #[case] map: Option<&str>,
) -> Result<(), String> {
    // Given
    let output = ExtractOutput {
        code: "const width = 4;".to_string(),
        styles: Default::default(),
        map: map.map(str::to_string),
        css_file: None,
        dependencies: Vec::new(),
    };
    // When
    let error = Unit::retained("retained.ts", &output, "const width: number = 4;")
        .err()
        .ok_or("missing map accepted")?;
    // Then
    assert!(error.starts_with("retained.ts:1:1: Cannot trace retained stylesheet code"));
    Ok(())
}

#[test]
#[serial]
fn retained_initializers_keep_original_sites_while_extracted_glue_has_only_file_fallback()
-> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = "import { css } from '@devup-ui/react';\nconst width: number = 4;\nexport function value() { return width; }\nexport const rule = css({ w: width });";
    let output =
        crate::extract_with_source_map("retained.ts", code, ExtractOption::default(), true, None)
            .map_err(|error| error.to_string())?;
    // When
    let unit = Unit::retained("retained.ts", &output, code)?;
    // Then
    assert_eq!(
        unit.place(
            unit.script()
                .find("= 4")
                .ok_or("missing retained initializer")?
                + 2
        ),
        "retained.ts:2:23"
    );
    assert_eq!(
        unit.locate(unit.script().find("rule").ok_or("missing extracted rule")?),
        None
    );
    Ok(())
}

#[path = "selected_validation_tests.rs"]
mod selected_validation_tests;
