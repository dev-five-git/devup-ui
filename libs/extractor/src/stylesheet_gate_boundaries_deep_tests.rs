use crate::{ExtractOption, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

#[test]
#[serial]
fn gate_boundaries_heap_deep_composition_uses_default_native_stack() -> Result<(), String> {
    // Given: shallow syntax, genuine 100000-deep acyclic heap arrays, below Boa's loop budget.
    reset_class_map();
    reset_file_map();
    let expected = extract_without_source_map(
        "/heap.css.ts",
        "import{css}from'@devup-ui/react';export const card=css({p:2,color:'red'});",
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())?;
    reset_class_map();
    reset_file_map();
    let source = "import{css}from'@devup-ui/react';let rules={p:2,color:'red'};for(let i=0;i<25000;i++){rules=[[[[rules]]]]}export const card=css(rules);";
    // When
    let output = extract_without_source_map("/heap.css.ts", source, ExtractOption::default())
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case(
    "let value=()=>1;for(let i=0;i<25000;i++){value=[[[[value]]]]}export const card=css({p:value});"
)]
#[case(
    "const first=[];let rules=first;for(let i=0;i<25000;i++){rules=[[[[rules]]]]}first.push(rules);export const card=css(rules);"
)]
#[case(
    "let value=1;for(let i=0;i<25000;i++){value=[[[[value]]]]}export const card=css({p:value});"
)]
#[serial]
fn gate_boundaries_heap_deep_invalid_data_returns_located_error(
    #[case] body: &str,
) -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import{{css}}from'@devup-ui/react';\n{body}");
    // When
    let error =
        extract_without_source_map("/heap-invalid.css.ts", &source, ExtractOption::default())
            .err()
            .ok_or("invalid heap data must reject")?
            .to_string();
    // Then
    assert!(error.starts_with("/heap-invalid.css.ts:2:"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn gate_boundaries_heap_deep_tag_composition_matches_flat_rules() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    let expected = extract_without_source_map(
        "/heap-tag.css.ts",
        "import{css}from'@devup-ui/react';export const card=css({p:2,color:'red'});",
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())?;
    reset_class_map();
    reset_file_map();
    let source = "import{css}from'@devup-ui/react';let rules={p:2,color:'red'};for(let i=0;i<25000;i++){rules=[[[[rules]]]]}export const card=css`${rules}`;";
    // When
    let output = extract_without_source_map("/heap-tag.css.ts", source, ExtractOption::default())
        .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn gate_boundaries_heap_deep_object_matches_flat_selector_at_default_stack() -> Result<(), String> {
    // Given
    reset_class_map();
    reset_file_map();
    let selector = format!("&{}", ":hover".repeat(100_000));
    let control = format!(
        "import{{css}}from'@devup-ui/react';export const card=css({{selectors:{{'{selector}':{{p:2}}}}}});"
    );
    let expected =
        extract_without_source_map("/heap-object.css.ts", &control, ExtractOption::default())
            .map_err(|error| error.to_string())?;
    reset_class_map();
    reset_file_map();
    let source = concat!(
        "import{",
        "css}from'@devup-ui/react';let rules={p:2};for(let i=0;i<25000;i++){rules={_hover:{_hover:{_hover:{_hover:rules}}}}}export const card=css(rules);"
    );
    // When
    let output =
        extract_without_source_map("/heap-object.css.ts", source, ExtractOption::default())
            .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}
