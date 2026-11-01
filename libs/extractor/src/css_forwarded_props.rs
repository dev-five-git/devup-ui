use crate::extractor::extract_style_from_styled::forwarding;
use oxc_allocator::TakeIn;
use oxc_ast::{
    ast::{Expression, JSXAttributeItem, JSXAttributeValue, ObjectExpression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;

pub(super) fn filter_object<'a>(
    ast: &AstBuilder<'a>,
    props: &mut ObjectExpression<'a>,
    denied: &[String],
) {
    if denied.is_empty() {
        return;
    }
    let original =
        Expression::new_object_expression(props.span, props.properties.take_in(ast), ast);
    // Copy config first so accessors run in original enumeration order on their original receiver.
    let copy = Expression::new_object_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(
            [ObjectPropertyKind::new_spread_property(SPAN, original, ast)],
            ast,
        ),
        ast,
    );
    let filtered = forwarding::without_props(ast, copy, denied);
    props
        .properties
        .push(ObjectPropertyKind::new_spread_property(SPAN, filtered, ast));
}

pub(super) fn filter_jsx<'a>(
    ast: &AstBuilder<'a>,
    attrs: &mut oxc_allocator::Vec<'a, JSXAttributeItem<'a>>,
    denied: &[String],
) {
    attrs.retain_mut(|item| {
        let JSXAttributeItem::Attribute(attribute) = item else {
            return true;
        };
        let Some(name) = attribute.name.as_identifier() else {
            return true;
        };
        if !denied.iter().any(|denied| denied == name.name.as_str()) {
            return true;
        }
        let value = match attribute.value.take() {
            Some(JSXAttributeValue::ExpressionContainer(mut container)) => container
                .expression
                .as_expression_mut()
                .map(|value| value.take_in(ast)),
            Some(JSXAttributeValue::Element(element)) => Some(Expression::JSXElement(element)),
            Some(JSXAttributeValue::Fragment(fragment)) => Some(Expression::JSXFragment(fragment)),
            Some(JSXAttributeValue::StringLiteral(_)) | None => None,
        };
        match value {
            None
            | Some(
                Expression::StringLiteral(_)
                | Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
                | Expression::BigIntLiteral(_),
            ) => false,
            Some(value) => {
                *item = JSXAttributeItem::new_spread_attribute(
                    SPAN,
                    Expression::new_unary_expression(SPAN, UnaryOperator::Void, value, ast),
                    ast,
                );
                true
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rstest::rstest]
    #[case("<div denied=<Child /> xml:lang='en' keep='yes' />", "<Child")]
    #[case("<div denied=<>text</> xml:lang='en' keep='yes' />", "<>text</>")]
    fn denied_jsx_value_when_it_is_an_element_retains_evaluation_and_namespaced_attributes(
        #[case] source: &str,
        #[case] evaluated: &str,
    ) {
        // Given
        let arena = oxc_allocator::Allocator::default();
        let ast = AstBuilder::new(&arena);
        let mut expression = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx())
            .parse_expression()
            .unwrap_or_else(|error| panic!("{error:?}"));
        let Expression::JSXElement(element) = &mut expression else {
            panic!("expected JSX")
        };
        // When
        filter_jsx(
            &ast,
            &mut element.opening_element.attributes,
            &["denied".into()],
        );
        // Then
        let JSXAttributeItem::SpreadAttribute(effect) = &element.opening_element.attributes[0]
        else {
            panic!("denied element creation must still execute")
        };
        assert!(
            matches!(&effect.argument, Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void)
        );
        let code = crate::utils::expression_to_code(&expression);
        assert!(!code.contains("denied="));
        assert!(code.contains(evaluated));
        assert!(code.contains("xml:lang=\"en\""));
        assert!(code.contains("keep=\"yes\""));
    }
}
