use crate::ExtractStyleProp;
use crate::extract_style::compiler_projection;
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use std::collections::BTreeMap;

#[path = "gen_style_conditional.rs"]
mod conditional;

#[derive(Clone, Debug)]
pub struct VariableUse {
    pub original: crate::extract_style::ExtractDynamicStyle,
    pub consumer: String,
    pub route: Option<String>,
    pub config: css::allocation_input::CapturedNameConfig,
    pub invocation: crate::extract_style::compiler_associations::InvocationId,
    pub scope: Option<crate::extract_style::compiler_associations::ScopeId>,
    pub receipts: Vec<crate::extract_style::compiler_receipts::ReceiptId>,
}
use crate::extract_style::compiler_projection::InlineAssignment;

pub fn gen_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    conditional::InlineProjection {
        ast: ast_builder,
        filename,
    }
    .styles(style_props)
}

fn gen_style<'a>(
    ast_builder: &AstBuilder<'a>,
    style: &ExtractStyleProp<'a>,
    filename: Option<&str>,
) -> Vec<ObjectPropertyKind<'a>> {
    let mut properties = vec![];
    if let ExtractStyleProp::Evaluated { binding, .. } = style {
        properties.push(ObjectPropertyKind::new_spread_property(
            SPAN,
            crate::assignment_lowering::projection(ast_builder, binding, 1),
            ast_builder,
        ));
    }
    if let ExtractStyleProp::Static(st) = style {
        if let Some(InlineAssignment {
            variable_name,
            identifier,
        }) = compiler_projection::inline(st, filename)
        {
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&variable_name, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&identifier, ast_builder.allocator()),
                    ast_builder,
                ),
                false,
                false,
                false,
                ast_builder,
            ));
        }
    } else if let ExtractStyleProp::StaticArray(res) = style {
        conditional::InlineProjection {
            ast: ast_builder,
            filename,
        }
        .extend_reversed(&mut properties, res);
    } else if let ExtractStyleProp::Conditional {
        condition,
        consequent,
        alternate,
    } = style
    {
        properties.extend(
            conditional::InlineProjection {
                ast: ast_builder,
                filename,
            }
            .conditional(condition, (consequent.as_deref(), alternate.as_deref())),
        );
    } else if let ExtractStyleProp::MemberExpression { map, expression } = style {
        let mut tmp_map = BTreeMap::<String, Vec<(String, String)>>::new();
        for (key, value) in map {
            for style in value.extract() {
                if let Some(InlineAssignment {
                    variable_name,
                    identifier,
                }) = compiler_projection::inline(&style, filename)
                {
                    tmp_map
                        .entry(variable_name)
                        .or_default()
                        .push((key.clone(), identifier));
                }
            }
        }

        for (key, value) in tmp_map {
            let v = if value.len() == 1 {
                // do not create object expression when property is single
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&value[0].1, ast_builder.allocator()),
                    ast_builder,
                )
            } else {
                Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    SPAN,
                    Expression::new_object_expression(
                        SPAN,
                        oxc_allocator::Vec::from_iter_in(
                            value.into_iter().map(|(k, v)| {
                                ObjectPropertyKind::new_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    PropertyKey::new_static_identifier(
                                        SPAN,
                                        Str::from_in(&k, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    Expression::new_identifier(
                                        SPAN,
                                        Str::from_in(&v, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    false,
                                    false,
                                    false,
                                    ast_builder,
                                )
                            }),
                            ast_builder,
                        ),
                        ast_builder,
                    ),
                    expression.clone_in(ast_builder.allocator()),
                    false,
                    ast_builder,
                ))
            };
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&key, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                v,
                false,
                false,
                false,
                ast_builder,
            ));
        }
    }
    properties
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_style_value::ExtractStyleValue,
    };
    use crate::utils::expression_to_code;
    use oxc_allocator::Allocator;
    use serial_test::serial;

    fn dynamic_style<'a>(property: &str, identifier: &str) -> ExtractStyleProp<'a> {
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            property, 0, identifier, None,
        )))
    }

    #[test]
    #[serial]
    fn test_gen_styles_for_alternate_only_conditional() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let styles = [ExtractStyleProp::Conditional {
            condition: Expression::new_identifier(SPAN, "enabled", &builder),
            consequent: None,
            alternate: Some(Box::new(dynamic_style("color", "fallbackColor"))),
        }];

        let generated = gen_styles(&builder, &styles, None).unwrap();
        let code = expression_to_code(&generated);

        assert!(code.contains("enabled?undefined:fallbackColor"));
    }

    #[test]
    #[serial]
    fn test_gen_styles_for_dynamic_member_expression() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let mut map = BTreeMap::new();
        map.insert(
            "primary".to_string(),
            Box::new(dynamic_style("color", "primaryColor")),
        );
        map.insert(
            "secondary".to_string(),
            Box::new(dynamic_style("color", "secondaryColor")),
        );
        let styles = [ExtractStyleProp::MemberExpression {
            map,
            expression: Expression::new_identifier(SPAN, "variant", &builder),
        }];

        let generated = gen_styles(&builder, &styles, None).unwrap();
        let code = expression_to_code(&generated);

        assert!(code.contains("primaryColor"));
        assert!(code.contains("secondaryColor"));
        assert!(code.contains("[variant]"));
    }

    #[test]
    #[serial]
    fn test_gen_styles_for_conditional_with_distinct_properties() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let styles = [ExtractStyleProp::Conditional {
            condition: Expression::new_identifier(SPAN, "enabled", &builder),
            consequent: Some(Box::new(dynamic_style("color", "enabledColor"))),
            alternate: Some(Box::new(dynamic_style("background", "disabledBackground"))),
        }];

        let generated = gen_styles(&builder, &styles, None).unwrap();
        let code = expression_to_code(&generated);

        assert!(code.contains("enabledColor"));
        assert!(code.contains("disabledBackground"));
    }
}
