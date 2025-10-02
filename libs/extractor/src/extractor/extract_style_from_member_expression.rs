use crate::{
    ExtractStyleProp,
    extractor::{
        ExtractResult,
        extract_style_from_expression::{dynamic_style, extract_style_from_expression},
    },
    utils::{get_number_by_literal_expression, get_string_by_literal_expression},
};
use css::style_selector::StyleSelector;
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{
        ArrayExpressionElement, ComputedMemberExpression, Expression, ObjectPropertyKind,
        PropertyKey,
    },
};
use oxc_span::SPAN;
use std::collections::BTreeMap;

pub(super) fn extract_style_from_member_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    name: Option<&str>,
    mem: &mut ComputedMemberExpression<'a>,
    level: u8,
    selector: &Option<StyleSelector>,
) -> ExtractResult<'a> {
    let mem_expression = &mem.expression.clone_in(ast_builder.allocator);
    let mut ret: Vec<ExtractStyleProp> = vec![];

    if let Expression::ArrayExpression(array) = &mut mem.object
        && !array.elements.is_empty()
    {
        if let Some(num) = get_number_by_literal_expression(mem_expression) {
            if num < 0f64 {
                return ExtractResult::default();
            }
            let mut etc = None;
            for (idx, p) in array.elements.iter_mut().enumerate() {
                if let ArrayExpressionElement::SpreadElement(sp) = p {
                    etc = Some(sp.argument.clone_in(ast_builder.allocator));
                } else if idx as f64 == num
                    && let Some(p) = p.as_expression_mut()
                {
                    return extract_style_from_expression(ast_builder, name, p, level, selector);
                }
            }
            return ExtractResult {
                props: None,
                styles: etc
                    .map(|etc| {
                        vec![dynamic_style(
                            ast_builder,
                            name.unwrap(),
                            &Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            ),
                            level,
                            selector,
                        )]
                    })
                    .unwrap_or_default(),
                tag: None,
                style_order: None,
                style_vars: None,
            };
        }

        let mut map = BTreeMap::new();
        for (idx, p) in array.elements.iter_mut().enumerate() {
            if let ArrayExpressionElement::SpreadElement(sp) = p {
                map.insert(
                    idx.to_string(),
                    Box::new(dynamic_style(
                        ast_builder,
                        name.unwrap(),
                        &Expression::ComputedMemberExpression(
                            ast_builder.alloc_computed_member_expression(
                                SPAN,
                                sp.argument.clone_in(ast_builder.allocator),
                                mem_expression.clone_in(ast_builder.allocator),
                                false,
                            ),
                        ),
                        level,
                        &selector.clone(),
                    )),
                );
            } else if let Some(p) = p.as_expression_mut() {
                map.insert(
                    idx.to_string(),
                    Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(ast_builder, name, p, level, selector).styles,
                    )),
                );
            }
        }

        ret.push(ExtractStyleProp::MemberExpression {
            expression: mem_expression.clone_in(ast_builder.allocator),
            map,
        });
    } else if let Expression::ObjectExpression(obj) = &mut mem.object
        && !obj.properties.is_empty()
    {
        let mut map = BTreeMap::new();
        if let Some(k) = get_string_by_literal_expression(mem_expression) {
            let mut etc = None;
            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(o) = p {
                    if let PropertyKey::StaticIdentifier(ref pk) = o.key
                        && pk.name == k
                    {
                        return ExtractResult {
                            styles: extract_style_from_expression(
                                ast_builder,
                                name,
                                &mut o.value,
                                level,
                                selector,
                            )
                            .styles,
                            ..ExtractResult::default()
                        };
                    }
                } else if let ObjectPropertyKind::SpreadProperty(sp) = p {
                    etc = Some(sp.argument.clone_in(ast_builder.allocator));
                }
            }

            match etc {
                None => return ExtractResult::default(),
                Some(etc) => ret.push(dynamic_style(
                    ast_builder,
                    name.unwrap(),
                    &Expression::ComputedMemberExpression(
                        ast_builder.alloc_computed_member_expression(
                            SPAN,
                            etc,
                            mem_expression.clone_in(ast_builder.allocator),
                            false,
                        ),
                    ),
                    level,
                    selector,
                )),
            }
        }

        for p in obj.properties.iter_mut() {
            if let ObjectPropertyKind::ObjectProperty(o) = p
                && let PropertyKey::StaticIdentifier(_)
                | PropertyKey::NumericLiteral(_)
                | PropertyKey::StringLiteral(_) = o.key
            {
                map.insert(
                    o.key.name().unwrap().to_string(),
                    Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(
                            ast_builder,
                            name,
                            &mut o.value,
                            level,
                            selector,
                        )
                        .styles,
                    )),
                );
            }
        }
        ret.push(ExtractStyleProp::MemberExpression {
            expression: mem_expression.clone_in(ast_builder.allocator),
            map,
        });
    } else if let Expression::Identifier(_) = &mut mem.object {
        ret.push(dynamic_style(
            ast_builder,
            name.unwrap(),
            &Expression::ComputedMemberExpression(ast_builder.alloc_computed_member_expression(
                SPAN,
                mem.object.clone_in(ast_builder.allocator),
                mem_expression.clone_in(ast_builder.allocator),
                false,
            )),
            level,
            selector,
        ))
    }

    ExtractResult {
        styles: ret,
        ..ExtractResult::default()
    }
}
