use super::*;

#[test]
fn template_when_opaque_callback_reads_unmapped_theme_retains_the_located_error() {
    // Given
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let expression = oxc_parser::Parser::new(
        &arena,
        "`${p=>p.theme.brand()};`",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::TemplateLiteral(template) = expression else {
        panic!("template")
    };
    let values = crate::style_values::StyleValues::default();
    let inline_css = rustc_hash::FxHashMap::default();
    let bindings = StyledBindings {
        imports: &|_| None,
        values: &values,
        inline_css: &inline_css,
    };
    // When
    let result = super::super::template_composition::compose(&ast, &template, &bindings);
    // Then
    assert_eq!(result.errors.len(), 1);
    assert!(result.errors[0].1.contains("theme"));
    assert_eq!(result.styles.len(), 0);
    assert_eq!(result.bindings.len(), 0);
}

#[test]
fn projected_selectors_when_key_is_dynamic_keep_the_refusal_in_both_views() {
    // Given
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut expression = oxc_parser::Parser::new(
        &arena,
        "({[selector]:{color:'red'}})",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    // When
    let (raw, authored) = crate::extractor::style_order_projection::extract_with_order(
        &ast, Some("selectors"), &mut expression, 0, &None,
        crate::extractor::extract_style_from_expression::LiteralHandling::ExpandResponsiveThemeToken,
    );
    // Then
    for props in [&raw.styles, &authored.0] {
        assert!(
            matches!(props.as_slice(), [crate::ExtractStyleProp::Unreadable{code,prop:false,..}] if code == "[selector]")
        );
    }
}
