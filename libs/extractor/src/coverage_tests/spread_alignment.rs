use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box className={css(...{bg: \"red\"})}/>;", "{bg: \"red\"}")]
#[case("<Box className={css(...{})}/>;", "{}")]
#[case("<Box className={css(...{...{bg: \"red\"}})}/>;", "{...{bg: \"red\"}}")]
#[case("export const j = css(...{ bg: 'red' });", "{ bg: 'red' }")]
#[serial]
fn legacy_object_spread_rejects_with_the_authored_call_and_fix(
    #[case] statement: &str,
    #[case] operand: &str,
) {
    // Given: preserve the original noniterable sources as rejection controls.
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");
    let offset = source
        .find("css(...")
        .unwrap_or_else(|| panic!("authored call"));
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_else(|| panic!("authored line"))
        .chars()
        .count()
        + 1;
    let allocator = oxc_allocator::Allocator::default();
    let expression = oxc_parser::Parser::new(&allocator, operand, oxc_span::SourceType::ts())
        .parse_expression()
        .unwrap_or_else(|error| panic!("valid spread operand: {error:?}"));
    let code = crate::utils::readable_code(&expression);
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let Err(actual) = crate::extract("spread.tsx", &source, crate::ExtractOption::default()) else {
        panic!("noniterable object argument spread must be rejected");
    };
    // Then
    assert_eq!(
        actual.to_string(),
        format!(
            "spread.tsx:{line}:{column}: Cannot compose `...{code}` at build time: each style must be a rule object, a class, or a condition choosing between them; pass the value itself instead of spreading it"
        )
    );
}
