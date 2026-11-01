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
