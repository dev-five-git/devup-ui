use super::literal_w38k_support::red;
use super::literal_w38l_c1c_source::produced;
use crate::utils::readable_code;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("", "css({color:'red'})")]
#[case("const mix=css({color:'red'});", "mix")]
#[serial]
fn styled_supplier_when_statement_has_real_singleton_provenance_separates_it(
    #[case] setup: &str,
    #[case] hole: &str,
) {
    // Given: the real imported producer runs under a source tag the visitor retains.
    let source = format!(
        "import {{css}} from '@devup-ui/react';{setup}unknown`color:blue;${{{hole}}};color:green`;"
    );
    let allocator = Allocator::default();
    let (visitor, mut expression) = produced(&allocator, &source);
    let Expression::TaggedTemplateExpression(tag) = &expression else {
        panic!("retained source template")
    };
    let original = &tag.quasi.expressions[0];
    let span = original.span();
    let text = readable_code(original);
    let reference = match original {
        Expression::Identifier(identifier) => Some(
            identifier
                .reference_id
                .get()
                .unwrap_or_else(|| panic!("producer reference ID")),
        ),
        Expression::StringLiteral(_) => None,
        other => panic!("actual finite producer result: {other:?}"),
    };
    let finite = visitor
        .style_values
        .finite(original)
        .unwrap_or_else(|| panic!("actual singleton provenance"))
        .clone();
    assert_eq!(finite.results.len(), 1);
    assert_eq!(finite.results[0].1, red(&[None]));
    let start = usize::try_from(span.start).unwrap_or_else(|error| panic!("span start: {error}"));
    let end = usize::try_from(span.end).unwrap_or_else(|error| panic!("span end: {error}"));
    assert_eq!(&source[start..end], hole);
    let default = crate::css_prop::template_parts(&visitor.ast, &tag.quasi, true)
        .unwrap_or_else(|error| panic!("default segments: {error:?}"));
    // When: the existing styled preparer consumes this same AST and visitor registry.
    visitor.prepare_styled_template(&mut expression);
    // Then: the actual result is a separate argument with original provenance/identity.
    let Expression::CallExpression(call) = &expression else {
        panic!("prepared styled call")
    };
    assert_eq!(call.arguments.len(), 3);
    let Some(actual) = call.arguments[1].as_expression() else {
        panic!("separate producer expression")
    };
    assert_eq!(actual.span(), span);
    assert_eq!(readable_code(actual), text);
    assert_eq!(visitor.style_values.finite(actual), Some(&finite));
    match (actual, reference) {
        (Expression::Identifier(identifier), Some(reference)) => {
            assert_eq!(identifier.reference_id.get(), Some(reference));
        }
        (Expression::StringLiteral(_), None) => {}
        other => panic!("changed producer identity: {other:?}"),
    }
    match hole {
        "css({color:'red'})" => {
            assert_eq!(default.len(), 1);
            assert!(matches!(&default[0], Expression::TemplateLiteral(_)));
        }
        "mix" => assert_eq!(default.len(), 3),
        other => panic!("fixture hole: {other}"),
    }
    let Some(leading) = call.arguments[0].as_expression() else {
        panic!("leading quasi expression")
    };
    assert_eq!(readable_code(leading), "`color:blue;`");
    let Some(trailing) = call.arguments[2].as_expression() else {
        panic!("trailing quasi expression")
    };
    assert_eq!(readable_code(trailing), "`;color:green`");
}
