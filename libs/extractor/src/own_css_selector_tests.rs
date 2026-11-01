use super::selector::SelectorText;
use css::style_selector::StyleSelector;

#[rstest::rstest]
#[case(&["&:nth-child", "&:hover", "=2n"])]
#[case(&["&:nth-child:nth-of-type", "=2n", "=odd"])]
#[case(&["&:nth-child", ":is(&)", "=2n"])]
#[case(&["&:hover", ":is([role=group],[data-group]):nth-child &", "=2n"])]
#[case(&["&:nth-child", "&[data-x='a:b']:nth-of-type", "=odd", "=2n"])]
#[case(&["&:nth-child, & > a", "&:hover", "=odd"])]
#[case(&["&:nth-child", "& &", "=2n", "=odd"])]
#[case(&["&:nth", "&-child > a", "=2n"])]
fn deep_object_selector_rope_matches_existing_selector_operations(#[case] operations: &[&str]) {
    // Given
    let mut expected = None;
    let mut actual = SelectorText::default();
    // When
    for operation in operations {
        if let Some(params) = operation.strip_prefix('=') {
            expected = expected.map(|selector| css::add_selector_params(selector, params));
            actual.params(params);
        } else {
            expected = Some(StyleSelector::nest_selector(expected.as_ref(), operation));
            actual.nest(operation);
        }
    }
    // Then
    assert_eq!(actual.text(), expected.map(|selector| selector.to_string()));
}

#[test]
fn deep_object_selector_rope_keeps_all_parameterized_occurrences() {
    // Given
    let mut selector = SelectorText::default();
    // When
    for _ in 0..100_000 {
        selector.nest("&:nth-child");
        selector.params("2n");
        selector.nest("&:hover");
    }
    // Then
    assert_eq!(
        selector.text(),
        Some(format!("&{}", ":nth-child(2n):hover".repeat(100_000)))
    );
}
