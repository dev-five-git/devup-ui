use crate::{
    ExtractStyleProp,
    extractor::{
        ExtractResult,
        extract_style_from_expression::{
            LiteralHandling, dynamic_style, extract_style_from_expression,
        },
    },
    utils::{
        get_number_by_literal_expression, get_str_by_property_key,
        get_string_by_literal_expression, get_string_by_property_key, readable_code,
    },
};
use css::style_selector::StyleSelector;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{ArrayExpressionElement, ComputedMemberExpression, Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::Span;
use std::collections::BTreeMap;

pub(super) fn extract_style_from_member_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    name: Option<&str>,
    mem: &mut ComputedMemberExpression<'a>,
    level: u8,
    selector: &Option<StyleSelector>,
) -> ExtractResult<'a> {
    let mem_expression = &mem.expression.clone_in(ast_builder.allocator());
    let mut ret: Vec<ExtractStyleProp> = vec![];

    // Unwrap type assertions and parenthesized expressions (e.g., `({...} as const)[key]`)
    while let Some(inner) = match &mem.object {
        Expression::TSAsExpression(ts_as) => {
            Some(ts_as.expression.clone_in(ast_builder.allocator()))
        }
        Expression::ParenthesizedExpression(p) => {
            Some(p.expression.clone_in(ast_builder.allocator()))
        }
        _ => None,
    } {
        mem.object = inner;
    }

    // With a spread in it, the literal as written gives what a position it
    // does not spell out holds
    let spread = match &mem.object {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_))),
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .any(|property| matches!(property, ObjectPropertyKind::SpreadProperty(_))),
        _ => false,
    }
    .then(|| mem.object.clone_in(ast_builder.allocator()));
    let span = mem.span;
    let runtime = |whole: &Expression<'a>| {
        runtime_member(
            ast_builder,
            name,
            whole.clone_in(ast_builder.allocator()),
            mem_expression,
            span,
            level,
            selector,
        )
    };

    if matches!(&mem.object, Expression::ArrayExpression(_))
        && spread.is_some()
        && get_number_by_literal_expression(mem_expression).is_none()
    {
        return ExtractResult {
            styles: vec![runtime(&mem.object)],
            ..ExtractResult::default()
        };
    }

    if let Expression::ArrayExpression(array) = &mut mem.object
        && !array.elements.is_empty()
    {
        if let Some(num) = get_number_by_literal_expression(mem_expression) {
            if num < 0f64 {
                return ExtractResult::default();
            }
            // Only the elements before the first spread sit at a fixed index
            let selected_index = (num.fract() == 0.0).then_some(num as usize);
            for (idx, p) in array.elements.iter_mut().enumerate() {
                if matches!(p, ArrayExpressionElement::SpreadElement(_)) {
                    break;
                }
                if Some(idx) == selected_index
                    && let Some(p) = p.as_expression_mut()
                {
                    return extract_style_from_expression(
                        ast_builder,
                        name,
                        p,
                        level,
                        selector,
                        LiteralHandling::ExpandResponsiveThemeToken,
                    );
                }
            }
            return ExtractResult {
                styles: spread.iter().map(runtime).collect(),
                ..ExtractResult::default()
            };
        }

        let mut map = BTreeMap::new();
        for (idx, p) in array.elements.iter_mut().enumerate() {
            if let Some(p) = p.as_expression_mut() {
                map.insert(
                    idx.to_string(),
                    Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(
                            ast_builder,
                            name,
                            p,
                            level,
                            selector,
                            LiteralHandling::ExpandResponsiveThemeToken,
                        )
                        .styles,
                    )),
                );
            }
        }

        ret.push(ExtractStyleProp::MemberExpression {
            expression: mem_expression.clone_in(ast_builder.allocator()),
            map,
        });
    } else if let Expression::ObjectExpression(obj) = &mut mem.object
        && !obj.properties.is_empty()
    {
        let mut map = BTreeMap::new();
        if let Some(k) = get_string_by_literal_expression(mem_expression) {
            // The last property written for the key gives it, unless a spread
            // after it may replace it
            let written = obj.properties.iter().rposition(|p| {
                matches!(p, ObjectPropertyKind::ObjectProperty(o)
                    if get_str_by_property_key(&o.key).as_deref() == Some(k.as_ref()))
            });
            let replaced = obj.properties[written.map_or(0, |index| index + 1)..]
                .iter()
                .any(|p| matches!(p, ObjectPropertyKind::SpreadProperty(_)));
            if replaced && let Some(whole) = &spread {
                return ExtractResult {
                    styles: vec![runtime(whole)],
                    ..ExtractResult::default()
                };
            }
            for p in obj.properties.iter_mut().rev() {
                if let ObjectPropertyKind::ObjectProperty(o) = p
                    && get_str_by_property_key(&o.key).as_deref() == Some(k.as_ref())
                {
                    return ExtractResult {
                        styles: extract_style_from_expression(
                            ast_builder,
                            name,
                            &mut o.value,
                            level,
                            selector,
                            LiteralHandling::ExpandResponsiveThemeToken,
                        )
                        .styles,
                        ..ExtractResult::default()
                    };
                }
            }
            return ExtractResult::default();
        }

        for p in &mut obj.properties {
            if let ObjectPropertyKind::ObjectProperty(o) = p
                && let Some(property_name) = get_string_by_property_key(&o.key)
            {
                map.insert(
                    property_name,
                    Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(
                            ast_builder,
                            name,
                            &mut o.value,
                            level,
                            selector,
                            LiteralHandling::ExpandResponsiveThemeToken,
                        )
                        .styles,
                    )),
                );
            }
        }
        ret.push(ExtractStyleProp::MemberExpression {
            expression: mem_expression.clone_in(ast_builder.allocator()),
            map,
        });
    } else if !matches!(
        &mem.object,
        Expression::ArrayExpression(_) | Expression::ObjectExpression(_)
    ) {
        ret.push(runtime_member(
            ast_builder,
            name,
            mem.object.clone_in(ast_builder.allocator()),
            mem_expression,
            span,
            level,
            selector,
        ));
    }

    ExtractResult {
        styles: ret,
        ..ExtractResult::default()
    }
}

/// `object[key]` known only at runtime: a CSS variable for a property, and
/// under a selector, which takes styles, what the build cannot read
fn runtime_member<'a>(
    ast_builder: &AstBuilder<'a>,
    name: Option<&str>,
    object: Expression<'a>,
    key: &Expression<'a>,
    span: Span,
    level: u8,
    selector: &Option<StyleSelector>,
) -> ExtractStyleProp<'a> {
    let member = Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
        span,
        object,
        key.clone_in(ast_builder.allocator()),
        false,
        ast_builder,
    ));
    match name {
        Some(name) => dynamic_style(ast_builder, name, &member, level, selector),
        None => ExtractStyleProp::Unreadable {
            offset: span.start,
            code: readable_code(&member),
            prop: false,
        },
    }
}
