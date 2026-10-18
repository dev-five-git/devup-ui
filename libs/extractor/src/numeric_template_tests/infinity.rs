use super::*;
use crate::utils::{
    get_number_by_literal_expression, get_string_by_literal_expression, js_number_string,
};

#[rstest]
#[case::lower_inf("`inf`")]
#[case::upper_inf("`INF`")]
#[case::mixed_inf("`iNf`")]
#[case::positive_inf("`+inf`")]
#[case::negative_inf("`-INF`")]
#[case::lower_infinity("`infinity`")]
#[case::upper_infinity("`INFINITY`")]
#[case::mixed_infinity("`iNfInItY`")]
#[case::positive_infinity("`+infinity`")]
#[case::negative_infinity("`-INFINITY`")]
#[case::assembled_alias("`in${'f'}`")]
#[case::unary_escaped_alias(r"+`\x69nf`")]
fn numeric_template_rejects_rust_only_infinity_aliases(#[case] source: &str) {
    // Given
    let arena = Allocator::default();
    let expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    // When
    let actual = get_number_by_literal_expression(&expression);
    // Then
    assert_eq!(actual, None, "{source}");
}

#[rstest]
#[case::canonical("`Infinity`", "Infinity")]
#[case::positive("`+Infinity`", "Infinity")]
#[case::negative("`-Infinity`", "-Infinity")]
#[case::escaped(r"`\x49nfinity`", "Infinity")]
#[case::assembled("`${'Infi'}nity`", "Infinity")]
#[case::overflow("`1e309`", "Infinity")]
#[case::negative_overflow("`-1e309`", "-Infinity")]
#[case::unary_negative("-`Infinity`", "-Infinity")]
fn numeric_template_preserves_supported_nonfinite_values(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given
    let arena = Allocator::default();
    let expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    // When
    let actual = get_number_by_literal_expression(&expression).map(js_number_string);
    // Then
    assert_eq!(actual.as_deref(), Some(expected), "{source}");
}

#[rstest]
#[case::escaped_alias(r"`\x69nf`", "inf")]
#[case::signed_alias("`-INF`", "-INF")]
#[case::cased_infinity("`infinity`", "infinity")]
#[case::signed_infinity("`+Infinity`", "+Infinity")]
#[case::overflow("`1e309`", "1e309")]
#[case::alias_substitution("`value:${'INF'}`", "value:INF")]
#[case::nested_alias(r"`value:${`\x69nf`}`", "value:inf")]
#[case::nested_overflow("`value:${`1e309`}`", "value:1e309")]
fn template_strings_preserve_spelling_without_unary_coercion(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given
    let arena = Allocator::default();
    let expression = Parser::new(&arena, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("invalid template fixture: {error:?}"));
    // When
    let actual = get_string_by_literal_expression(&expression);
    // Then
    assert_eq!(actual.as_deref(), Some(expected), "{source}");
}
