use super::*;
use crate::css_utils::Place;
use rstest::rstest;

#[rstest]
#[case("`&${'.active'};`", "selector")]
#[case("`color: ${'red'};`", "value")]
#[case("`${'color: red;'};`", "statement")]
fn literal_prefix_when_nonempty_is_selector_context_not_value_or_statement(
    #[case] source: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let expression = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let template = match expression {
        Expression::TemplateLiteral(template) => Some(template),
        _ => None,
    }
    .ok_or("expected template fixture")?;
    // When
    let actual = place(&template.quasis[0].value.raw, &template.quasis[1..]);
    // Then
    assert_eq!(
        match actual {
            Place::Other => "selector",
            Place::Value => "value",
            Place::Statement => "statement",
        },
        expected
    );
    Ok(())
}

#[test]
fn unknown_suffix_when_selector_prefix_is_nonempty_preserves_interpolation_index()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = "`&${suffix};`";
    let expression = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    let template = match expression {
        Expression::TemplateLiteral(template) => Some(template),
        _ => None,
    }
    .ok_or("expected template fixture")?;
    // When
    let parts = split(&ast, &template, |_| false);
    // Then
    assert!(matches!(
        parts.as_slice(),
        [Part::Unplaced(0), Part::Text(_)]
    ));
    let text = match &parts[1] {
        Part::Text(text) => Some(text),
        Part::Mixin { .. } | Part::Unplaced(_) => None,
    }
    .ok_or("expected preserved template text")?;
    assert_eq!(text.quasis[0].value.raw, "&");
    assert_eq!(text.quasis[1].value.raw, ";");
    assert_eq!(crate::utils::readable_code(&text.expressions[0]), "suffix");
    Ok(())
}
