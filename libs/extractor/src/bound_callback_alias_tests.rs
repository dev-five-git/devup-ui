use std::collections::HashMap;

use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[path = "bound_callback_export_tests.rs"]
mod exports;

#[path = "bound_callback_numeric_tests.rs"]
mod numeric;

#[rstest]
#[case("p => ({'@layer': {base: {color:p.color}}})", true)]
#[case("function(p) { return {'@layer': {base: {color:p.color}}}; }", true)]
#[case("p => ({'@layer': {inherit: {color:p.color}}})", false)]
#[case(
    "function(p) { return {'@layer': {inherit: {color:p.color}}}; }",
    false
)]
#[serial]
fn compiles_callback_layer_qa_sources(
    #[case] callback: &str,
    #[case] valid: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import styled from '@emotion/styled';\nconst rules = {callback}; export const Choice = styled.div(rules);"
    );
    let option = ExtractOption {
        import_aliases: HashMap::from([(
            "@emotion/styled".into(),
            ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..ExtractOption::default()
    };
    // When
    let result = extract("callback-layer.tsx", &source, option);
    // Then
    if valid {
        let output = result?;
        assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Dynamic(style) if style.layer.as_deref() == Some("base"))));
        assert!(output.code.contains("\"--"), "{}", output.code);
    } else {
        let Err(error) = result else {
            panic!("expected invalid layer error");
        };
        let error = error.to_string();
        let line = source
            .lines()
            .nth(1)
            .ok_or("missing callback source line")?;
        let column = line.find("inherit").ok_or("missing inherit key")? + 1;
        assert!(
            error.starts_with(&format!("callback-layer.tsx:2:{column}:")),
            "{error}"
        );
        assert!(error.contains("@layer inherit"), "{error}");
    }
    Ok(())
}

#[rstest]
#[case("import * as ui from '@devup-ui/react';", "ui.styled.div")]
#[case("import {styled as make} from '@devup-ui/react';", "make.div")]
#[serial]
fn resolves_semantic_style_imports(
    #[case] import: &str,
    #[case] callee: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "{import} const rules = p => ({{color:p.color}}); export const C = {callee}(rules);"
    );
    // When
    let output = extract("callback.tsx", &source, ExtractOption::default())?;
    // Then
    assert!(
        output
            .styles
            .iter()
            .any(|style| matches!(style, ExtractStyleValue::Dynamic(_))),
        "{output:?}"
    );
    Ok(())
}
