use super::literal_w38l_c1c_source::produced;
use crate::css_prop::template_parts;
use crate::utils::readable_code;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("${'margin:0'}", None, 1)]
#[case("${'a'};", None, 1)]
#[case("color:${css({color:'red'})};", Some(1), 1)]
#[case("&:hover{${css({color:'red'})};}", Some(1), 1)]
#[case("${base};", None, 2)]
#[case("${p=>({color:'red'})};", None, 2)]
#[case("${css({color:active?'red':'blue'})};", Some(2), 2)]
#[case("", None, 0)]
#[case(";", None, 1)]
#[case("/* ;{} */content:';{}';", None, 1)]
#[serial]
fn styled_supplier_when_hole_is_ineligible_preserves_default_segments(
    #[case] body: &str,
    #[case] finite_count: Option<usize>,
    #[case] count: usize,
) {
    // Given: all holes and any finite provenance come from parsed/visited source.
    let source = format!("import {{css}} from '@devup-ui/react';unknown`{body}`;");
    let allocator = Allocator::default();
    let (visitor, mut expression) = produced(&allocator, &source);
    let Expression::TaggedTemplateExpression(tag) = &expression else {
        panic!("retained source template")
    };
    let actual_count = tag
        .quasi
        .expressions
        .first()
        .and_then(|hole| visitor.style_values.finite(hole))
        .map(|finite| finite.results.len());
    assert_eq!(actual_count, finite_count);
    let default = template_parts(&visitor.ast, &tag.quasi, true)
        .unwrap_or_else(|error| panic!("default segments: {error:?}"));
    // When: the unchanged public/default entry and existing styled supplier consume it.
    visitor.prepare_styled_template(&mut expression);
    // Then: outside singleton statements the exact segments are retained.
    assert_eq!(default.len(), count);
    let Expression::CallExpression(call) = &expression else {
        panic!("prepared call")
    };
    let actual = call
        .arguments
        .iter()
        .map(|argument| {
            let Some(value) = argument.as_expression() else {
                panic!("prepared argument expression")
            };
            (value.span(), readable_code(value))
        })
        .collect::<Vec<_>>();
    let expected = default
        .iter()
        .map(|value| (value.span(), readable_code(value)))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert_eq!(visitor.errors, vec![]);
}
