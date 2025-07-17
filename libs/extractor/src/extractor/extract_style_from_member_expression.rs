use crate::{
    ExtractStyleProp,
    extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_style_value::ExtractStyleValue,
    },
    extractor::{ExtractResult, extract_style_from_expression::extract_style_from_expression},
    utils::{
        expression_to_code, get_number_by_literal_expression, get_string_by_literal_expression,
    },
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

    match &mut mem.object {
        Expression::ArrayExpression(array) => {
            if array.elements.is_empty() {
                return ExtractResult::default();
            }

            if let Some(num) = get_number_by_literal_expression(mem_expression) {
                if num < 0f64 {
                    return ExtractResult::default();
                }
                let mut etc = None;
                for (idx, p) in array.elements.iter_mut().enumerate() {
                    if let ArrayExpressionElement::SpreadElement(sp) = p {
                        etc = Some(sp.argument.clone_in(ast_builder.allocator));
                        continue;
                    }
                    if idx as f64 == num {
                        return extract_style_from_expression(
                            ast_builder,
                            name,
                            p.to_expression_mut(),
                            level,
                            selector,
                        );
                    }
                }
                return ExtractResult {
                    styles: etc
                        .map(|etc| {
                            vec![ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                                ExtractDynamicStyle::new(
                                    name.unwrap(),
                                    level,
                                    &expression_to_code(&Expression::ComputedMemberExpression(
                                        ast_builder.alloc_computed_member_expression(
                                            SPAN,
                                            etc,
                                            mem_expression.clone_in(ast_builder.allocator),
                                            false,
                                        ),
                                    )),
                                    selector.clone(),
                                ),
                            ))]
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
                        Box::new(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                            ExtractDynamicStyle::new(
                                name.unwrap(),
                                level,
                                &expression_to_code(&Expression::ComputedMemberExpression(
                                    ast_builder.alloc_computed_member_expression(
                                        SPAN,
                                        sp.argument.clone_in(ast_builder.allocator),
                                        mem_expression.clone_in(ast_builder.allocator),
                                        false,
                                    ),
                                )),
                                selector.clone(),
                            ),
                        ))),
                    );
                } else {
                    map.insert(
                        idx.to_string(),
                        Box::new(ExtractStyleProp::StaticArray(
                            extract_style_from_expression(
                                ast_builder,
                                name,
                                p.to_expression_mut(),
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
        }
        Expression::ObjectExpression(obj) => {
            if obj.properties.is_empty() {
                return ExtractResult::default();
            }

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
                    None => {
                        return ExtractResult::default();
                    }
                    Some(etc) => ret.push(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                        ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            &expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            )),
                            selector.clone(),
                        ),
                    ))),
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
        }
        Expression::Identifier(_) => {
            if let Some(name) = name {
                ret.push(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                    ExtractDynamicStyle::new(
                        name,
                        level,
                        &expression_to_code(&Expression::ComputedMemberExpression(
                            ast_builder.alloc_computed_member_expression(
                                SPAN,
                                mem.object.clone_in(ast_builder.allocator),
                                mem_expression.clone_in(ast_builder.allocator),
                                false,
                            ),
                        )),
                        selector.clone(),
                    ),
                )))
            }
        }
        _ => {}
    };

    ExtractResult {
        styles: ret,
        ..ExtractResult::default()
    }
}
