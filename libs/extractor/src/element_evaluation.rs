use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElement, Str},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, GetSpanMut, SPAN};

use crate::{
    ExtractStyleProp, ExtractStyleValue,
    utils::{get_string_by_literal_expression, stays_attribute},
};

#[path = "element_capture.rs"]
mod element_capture;
pub(crate) use element_capture::{capture, default_type};

#[path = "element_typography.rs"]
mod element_typography;
pub(crate) use element_typography::capture as typography;

#[cfg(test)]
#[path = "assignment_container_tests.rs"]
mod assignment_container_tests;

#[cfg(test)]
#[path = "assignment_mixed_container_tests.rs"]
mod assignment_mixed_container_tests;

pub(crate) fn needed(element: &JSXElement<'_>) -> bool {
    let values = element.opening_element.attributes.iter().filter(|attribute| matches!(attribute,
        JSXAttributeItem::Attribute(attribute) if matches!(attribute.value, Some(JSXAttributeValue::ExpressionContainer(_))))).count();
    element.opening_element.attributes.iter().any(|attribute| matches!(attribute, JSXAttributeItem::SpreadAttribute(_))) || element.opening_element.attributes.iter().any(|attribute| matches!(attribute,
        JSXAttributeItem::Attribute(attribute)
         if matches!(&attribute.name, JSXAttributeName::Identifier(name) if !stays_attribute(&name.name) || name.name == "className")
        && matches!(&attribute.value, Some(JSXAttributeValue::ExpressionContainer(container))
            if container.expression.as_expression().is_some_and(|value| get_string_by_literal_expression(value).is_none()))))
        && (values > 1 || !element.children.is_empty())
}

pub(crate) fn raw_typography<'a>(ast: &AstBuilder<'a>, source: &Expression<'a>) -> Expression<'a> {
    use oxc_ast::ast::{TemplateElement, TemplateElementValue};
    let span = source.span();
    let value = Expression::new_identifier(span, "__devupTypography", ast);
    let class = Expression::new_conditional_expression(
        span,
        value.clone_in(ast.allocator()),
        Expression::new_template_literal(
            span,
            oxc_allocator::Vec::from_array_in(
                [
                    TemplateElement::new(
                        span,
                        TemplateElementValue {
                            raw: Str::from("typo-"),
                            cooked: None,
                        },
                        false,
                        ast,
                    ),
                    TemplateElement::new(
                        span,
                        TemplateElementValue {
                            raw: Str::from(""),
                            cooked: None,
                        },
                        true,
                        ast,
                    ),
                ],
                ast,
            ),
            oxc_allocator::Vec::from_array_in([value], ast),
            ast,
        ),
        Expression::new_string_literal(span, "", None, ast),
        ast,
    );
    let mut result = crate::utils::call_with_values(
        ast,
        vec![(
            "__devupTypography".to_string(),
            source.clone_in(ast.allocator()),
        )],
        class,
    );
    *result.span_mut() = span;
    result
}

pub(crate) fn snapshot<'a>(ast: &AstBuilder<'a>, value: &Expression<'a>) -> Expression<'a> {
    let body = Expression::new_object_expression(
        value.span(),
        oxc_allocator::Vec::from_array_in(
            [oxc_ast::ast::ObjectPropertyKind::new_spread_property(
                value.span(),
                Expression::new_identifier(SPAN, "__devupSpreadValue", ast),
                ast,
            )],
            ast,
        ),
        ast,
    );
    let mut result = crate::utils::call_with_values(
        ast,
        vec![(
            "__devupSpreadValue".to_string(),
            value.clone_in(ast.allocator()),
        )],
        body,
    );
    *result.span_mut() = value.span();
    result
}

pub(crate) fn scalar<'a>(
    ast: &AstBuilder<'a>,
    styles: &mut [ExtractStyleProp<'a>],
    source: &Expression<'a>,
    next: &mut usize,
) -> Option<(String, Expression<'a>)> {
    if !styles.is_empty()
        && matches!(
            crate::utils::unwrap_syntax_only(source),
            Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
        )
    {
        let mut consumers = crate::assignment_owner::take_consumers(source.span(), styles);
        crate::assignment_consumers::merge(&mut consumers);
        if !consumers.is_empty() {
            let remaining =
                std::mem::replace(&mut styles[0], ExtractStyleProp::StaticArray(vec![]));
            styles[0] = ExtractStyleProp::StaticArray(vec![
                remaining,
                ExtractStyleProp::Evaluated {
                    binding: crate::sparse_sites::binding_name(source.span().start),
                    styles: consumers,
                    source: source.clone_in(ast.allocator()),
                    evaluation: None,
                    alternate_order: None,
                    alternate_class: false,
                },
            ]);
        }
        return None;
    }
    if !styles.iter().all(|style| {
        matches!(
            style,
            ExtractStyleProp::Static(ExtractStyleValue::Dynamic(_))
        )
    }) || styles.is_empty()
    {
        return None;
    }
    let name = format!("__devupSpread{next}");
    *next += 1;
    let input = styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style))
                if crate::source_normalization::suffix(ast, source, style).is_some() =>
            {
                Some(Expression::new_identifier(
                    source.span(),
                    Str::from_in(style.identifier(), ast.allocator()),
                    ast,
                ))
            }
            _ => None,
        })
        .unwrap_or_else(|| source.clone_in(ast.allocator()));
    for style in styles {
        if let ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style)) = style {
            style.replace_identifier(&name);
        }
    }
    Some((name, input))
}
