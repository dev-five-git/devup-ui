use super::*;

#[rstest::rstest]
#[case::null("null", "red")]
#[case::undefined("undefined", "red")]
#[case::value("'blue'", "blue")]
#[serial_test::serial]
fn authored_branches_when_nullish_choose_the_matching_color(
    #[case] tone: &str,
    #[case] expected: &str,
) {
    // Given: nullish extraction visits the fallback before the authored left operand.
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let mut expression =
        oxc_parser::Parser::new(&arena, "tone??'red'", oxc_span::SourceType::mjs())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
    // When: authored projection reconstructs the actual condition and its two sides.
    let (_, authored) = extract_with_order(
        &ast,
        Some("color"),
        &mut expression,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    let [
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        },
    ] = authored.0.as_slice()
    else {
        panic!("expected projected nullish condition")
    };
    let code = format!(
        "const tone={tone};{}",
        crate::utils::expression_to_code(condition)
    );
    let selected = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&code))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_boolean()
        .unwrap_or_else(|| panic!("condition must be boolean"));
    let side = if selected { consequent } else { alternate };
    let values: Vec<_> = side
        .as_ref()
        .unwrap_or_else(|| panic!("selected side must exist"))
        .extract()
        .into_iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => style.value,
            ExtractStyleValue::Dynamic(style) => {
                let code = format!("const tone={tone};{}", style.identifier());
                boa_engine::Context::default()
                    .eval(boa_engine::Source::from_bytes(&code))
                    .unwrap_or_else(|error| panic!("{error}"))
                    .as_string()
                    .unwrap_or_else(|| panic!("color must be a string"))
                    .to_std_string_escaped()
            }
            _ => panic!("expected color atom"),
        })
        .collect();
    // Then: fallback and dynamic values are never attached to each other's condition side.
    assert_eq!(values, vec![expected]);
}
