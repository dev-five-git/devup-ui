use crate::vanilla_extract::Stylesheet;
use rstest::rstest;

#[rstest]
#[case::namespace("<svg:path />", "Namespace tags are not supported")]
#[case::valueless_key("<div key />", "Please provide an explicit key value")]
fn component_diagnostic_when_lowering_real_tsx_keeps_location_and_fix(
    #[case] expression: &str,
    #[case] diagnostic: &str,
) -> Result<(), String> {
    // Given: the actual TSX lowering boundary, not a generated AST fixture.
    let source = format!("\n{expression}");
    let stylesheet = Stylesheet {
        filename: "/component.tsx",
        code: &source,
        source: &source,
        edits: &[],
    };
    // When
    let error = super::super::lower(expression, "__devupCreateElement")
        .err()
        .ok_or("Oxc diagnostic was discarded")?
        .located(
            stylesheet,
            oxc_span::Span::new(
                1,
                u32::try_from(source.len()).map_err(|error| error.to_string())?,
            ),
        );
    // Then
    assert!(
        error.starts_with("/component.tsx:2:1: Cannot compile component:"),
        "{error}"
    );
    assert!(error.contains(diagnostic), "{error}");
    assert!(error.ends_with("Fix: correct its expression"), "{error}");
    Ok(())
}

#[test]
fn component_diagnostic_when_expression_is_malformed_keeps_parser_details() -> Result<(), String> {
    // Given
    let expression = "<div>{}</div> +";
    // When
    let error = super::super::lower(expression, "__devupCreateElement")
        .err()
        .ok_or("malformed expression was accepted")?
        .to_string();
    // Then
    assert!(
        error.starts_with("Cannot compile component: Diagnostics([OxcDiagnostic"),
        "{error}"
    );
    assert!(error.contains("Unexpected token"), "{error}");
    assert!(error.ends_with("Fix: correct its expression"), "{error}");
    Ok(())
}
