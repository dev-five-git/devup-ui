use super::reject;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[test]
fn rejection_when_expression_arrow_contains_metadata_reports_original_key() {
    for key in ["styleOrder", "'style-order'"] {
        // Given
        let source = format!("() => ({{from:{{{key}:2,opacity:0}}}})");
        let allocator = Allocator::default();
        let expression = Parser::new(&allocator, &source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let offset = u32::try_from(source.find(key).unwrap_or_else(|| panic!("fixture key")))
            .unwrap_or_else(|error| panic!("{error}"));
        let mut errors = Vec::new();
        // When
        reject(&expression, "keyframes", &mut errors);
        // Then
        assert_eq!(
            errors,
            vec![crate::style_order::no_effect("keyframes", offset)]
        );
    }
}

#[test]
fn rejection_when_expression_arrow_has_only_declarations_leaves_audit_empty() {
    // Given
    let source = "() => ({from:{opacity:0}})";
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut errors = Vec::new();
    // When
    reject(&expression, "keyframes", &mut errors);
    // Then
    assert_eq!(errors, vec![]);
}

#[test]
fn rejection_when_block_arrow_contains_metadata_leaves_audit_empty() {
    for key in ["styleOrder", "'style-order'"] {
        // Given
        let source = format!("() => {{return {{from:{{{key}:2,opacity:0}}}};}}");
        let allocator = Allocator::default();
        let expression = Parser::new(&allocator, &source, SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let mut errors = Vec::new();
        // When
        reject(&expression, "keyframes", &mut errors);
        // Then: this audit does not establish public callback support.
        assert_eq!(errors, vec![]);
    }
}
