use super::*;
use oxc_span::GetSpan;

#[test]
fn discarded_functions_when_multiple_theme_reads_fail_keep_the_first_error() {
    // Given: two independently invalid nested callbacks in authored traversal order.
    let arena = oxc_allocator::Allocator::default();
    let source = "[p=>p.theme.colors[key],q=>q.theme.other[key]]";
    let expression = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::ArrayExpression(array) = &expression else {
        panic!("expected callback array")
    };
    let first = array.elements[0]
        .as_expression()
        .unwrap_or_else(|| panic!("expected first callback"));
    let expected = unmapped_theme_error("styled", first);
    let mut functions = Functions(None);
    // When: the real visitor encounters both callback expressions.
    functions.visit_expression(&expression);
    // Then: later callbacks cannot replace the original located theme diagnostic.
    assert_eq!(functions.0, Some(expected));
    assert_eq!(first.span().start, 1);
}

#[rstest::rstest]
#[case("enabled && {color:p.theme.colors.primary}", "var(--colors-primary)")]
#[case(
    "({color:enabled ? p.theme.colors.primary : p.theme.colors.secondary})",
    "var(--colors-secondary)"
)]
fn structured_theme_leaves_keep_the_original_controller(
    #[case] source: &str,
    #[case] expected: &str,
) {
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let params = oxc_parser::Parser::new(&arena, "p=>0", oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::ArrowFunctionExpression(function) = params else {
        panic!("expected props callback")
    };
    let original = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let before = crate::utils::readable_code(&original);
    let mapped = mapped_rules(&ast, &function.params, &original)
        .unwrap_or_else(|_| panic!("theme leaves must be mapped without mapping the controller"));
    let code = crate::utils::readable_code(&mapped);
    assert!(code.contains(expected), "{code}");
    assert!(code.contains("enabled"), "{code}");
    assert!(!code.contains("p.theme"), "{code}");
    assert_eq!(crate::utils::readable_code(&original), before);
    if source.contains('?') {
        assert!(code.contains("var(--colors-primary)"), "{code}");
        assert!(code.contains('?'), "{code}");
    } else {
        assert!(code.contains("&&"), "{code}");
    }
}

#[rstest::rstest]
#[case("p.theme.colors.primary && {color:'red'}")]
#[case("({color:p.theme.colors.primary ? 'red' : 'blue'})")]
fn structured_theme_controllers_remain_located_errors(#[case] source: &str) {
    let arena = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&arena);
    let params = oxc_parser::Parser::new(&arena, "p=>0", oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::ArrowFunctionExpression(function) = params else {
        panic!("expected props callback")
    };
    let original = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let expected = oxc_parser::Parser::new(
        &arena,
        "p=>p.theme.colors.primary",
        oxc_span::SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Err(PropsRulesError::Theme((offset, message))) =
        mapped_rules(&ast, &function.params, &original)
    else {
        panic!("a theme-dependent controller must be rejected")
    };
    let expected_offset = u32::try_from(
        source
            .find("p.theme")
            .unwrap_or_else(|| panic!("missing condition")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(offset, expected_offset);
    assert_eq!(message, unmapped_theme_error("styled", &expected).1);
}
