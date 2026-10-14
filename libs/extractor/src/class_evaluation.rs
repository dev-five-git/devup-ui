use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{BinaryOperator, Expression, LogicalOperator, Str, UnaryOperator},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, GetSpanMut, SPAN};
use oxc_syntax::number::NumberBase;

use crate::{
    assignment_lowering::projection,
    utils::{call_with_values, unwrap_syntax_only},
};

#[path = "class_template.rs"]
mod class_template;

pub(super) struct ClassEvaluation<'b, 'a>(pub(super) &'b AstBuilder<'a>);

pub(crate) fn capture<'a>(
    ast: &AstBuilder<'a>,
    source: &Expression<'a>,
    prepared: &mut [Option<Expression<'a>>],
) -> Option<(String, Expression<'a>)> {
    if matches!(unwrap_syntax_only(source), Expression::StringLiteral(_)) {
        return None;
    }
    let name = format!("__devupClass{}", source.span().start);
    let mut value = match prepared {
        [Some(primary), Some(alternate)] => {
            ClassEvaluation(ast).lower(source, [primary, alternate])
        }
        [Some(primary)] => primary.clone_in(ast.allocator()),
        _ => return None,
    };
    *value.span_mut() = source.span();
    let joint = prepared.len() == 2;
    for (index, prepared) in prepared.iter_mut().enumerate() {
        *prepared = Some(if joint {
            projection(ast, &name, if index == 0 { 0 } else { 3 })
        } else {
            Expression::new_identifier(
                source.span(),
                Str::from_in(name.as_str(), ast.allocator()),
                ast,
            )
        });
    }
    Some((name, value))
}

impl<'a> ClassEvaluation<'_, 'a> {
    pub(super) fn lower(
        &self,
        source: &Expression<'a>,
        variants: [&Expression<'a>; 2],
    ) -> Expression<'a> {
        let ast = self.0;
        let source = unwrap_syntax_only(source);
        let [primary, alternate] = variants.map(unwrap_syntax_only);
        match (source, primary, alternate) {
            (Expression::StringLiteral(_), _, _) => self.tuple(
                [
                    primary.clone_in(ast.allocator()),
                    source.clone_in(ast.allocator()),
                    alternate.clone_in(ast.allocator()),
                ],
                false,
            ),
            (
                Expression::ConditionalExpression(source),
                Expression::ConditionalExpression(primary),
                Expression::ConditionalExpression(alternate),
            ) => Expression::new_conditional_expression(
                source.span,
                source.test.clone_in(ast.allocator()),
                self.lower(
                    &source.consequent,
                    [&primary.consequent, &alternate.consequent],
                ),
                self.lower(
                    &source.alternate,
                    [&primary.alternate, &alternate.alternate],
                ),
                ast,
            ),
            (
                Expression::LogicalExpression(source),
                Expression::LogicalExpression(primary),
                Expression::LogicalExpression(alternate),
            ) => {
                let left = self.lower(&source.left, [&primary.left, &alternate.left]);
                let raw = projection(ast, "__devupClassLeft", 2);
                let test = match source.operator {
                    LogicalOperator::And => {
                        Expression::new_unary_expression(SPAN, UnaryOperator::LogicalNot, raw, ast)
                    }
                    LogicalOperator::Or => raw,
                    LogicalOperator::Coalesce => Expression::new_logical_expression(
                        SPAN,
                        Expression::new_binary_expression(
                            SPAN,
                            raw.clone_in(ast.allocator()),
                            BinaryOperator::StrictInequality,
                            Expression::new_null_literal(SPAN, ast),
                            ast,
                        ),
                        LogicalOperator::And,
                        Expression::new_binary_expression(
                            SPAN,
                            raw,
                            BinaryOperator::StrictInequality,
                            Expression::new_unary_expression(
                                SPAN,
                                UnaryOperator::Void,
                                Expression::new_numeric_literal(
                                    SPAN,
                                    0.0,
                                    None,
                                    NumberBase::Decimal,
                                    ast,
                                ),
                                ast,
                            ),
                            ast,
                        ),
                        ast,
                    ),
                };
                let selected = call_with_values(
                    ast,
                    vec![("__devupClassLeft".to_string(), left)],
                    Expression::new_conditional_expression(
                        SPAN,
                        test,
                        Expression::new_identifier(SPAN, "__devupClassLeft", ast),
                        Expression::new_null_literal(SPAN, ast),
                        ast,
                    ),
                );
                Expression::new_logical_expression(
                    source.span,
                    selected,
                    LogicalOperator::Coalesce,
                    self.lower(&source.right, [&primary.right, &alternate.right]),
                    ast,
                )
            }
            (
                Expression::TemplateLiteral(source),
                Expression::TemplateLiteral(primary),
                Expression::TemplateLiteral(alternate),
            ) => self.template(source, [primary, alternate]),
            _ => {
                let value = Expression::new_identifier(SPAN, "__devupClassValue", ast);
                call_with_values(
                    ast,
                    vec![(
                        "__devupClassValue".to_string(),
                        source.clone_in(ast.allocator()),
                    )],
                    self.tuple(
                        [
                            value.clone_in(ast.allocator()),
                            value.clone_in(ast.allocator()),
                            value,
                        ],
                        true,
                    ),
                )
            }
        }
    }

    pub(super) fn tuple(&self, values: [Expression<'a>; 3], shared: bool) -> Expression<'a> {
        let [primary, raw, alternate] = values;
        Expression::new_array_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                [
                    primary,
                    Expression::new_boolean_literal(SPAN, shared, self.0),
                    raw,
                    alternate,
                ]
                .into_iter()
                .map(oxc_ast::ast::ArrayExpressionElement::from),
                self.0,
            ),
            self.0,
        )
    }
}
