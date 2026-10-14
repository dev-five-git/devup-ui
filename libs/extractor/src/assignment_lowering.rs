use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        BinaryOperator, ChainElement, ComputedMemberExpression, Expression, LogicalOperator, Str,
        UnaryOperator,
    },
    builder::AstBuilder,
};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::number::NumberBase;

pub(crate) use crate::assignment_capture::{
    class_expression, coalesce, component, orders, take_evaluations,
};
use crate::{
    ExtractStyleProp,
    utils::{call_with_values, unwrap_syntax_only},
};

pub(crate) fn styled_argument<'a>(
    ast: &AstBuilder<'a>,
    source: &Expression<'a>,
    styles: &mut Vec<ExtractStyleProp<'a>>,
) {
    if matches!(
        unwrap_syntax_only(source),
        Expression::ConditionalExpression(_)
            | Expression::LogicalExpression(_)
            | Expression::ComputedMemberExpression(_)
    ) && !crate::static_assignment::literal_source(source)
    {
        *styles = vec![ExtractStyleProp::Evaluated {
            binding: crate::sparse_sites::binding_name(source.span().start),
            styles: std::mem::take(styles),
            source: source.clone_in(ast.allocator()),
            evaluation: None,
            alternate_order: None,
            alternate_class: false,
        }];
    }
}

pub(crate) fn projection<'a>(ast: &AstBuilder<'a>, binding: &str, slot: u8) -> Expression<'a> {
    let value = Expression::new_chain_expression(
        SPAN,
        ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
            SPAN,
            Expression::new_identifier(SPAN, Str::from_in(binding, ast.allocator()), ast),
            Expression::new_numeric_literal(SPAN, f64::from(slot), None, NumberBase::Decimal, ast),
            true,
            ast,
        )),
        ast,
    );
    if matches!(slot, 0 | 3) {
        Expression::new_logical_expression(
            SPAN,
            value,
            LogicalOperator::Coalesce,
            Expression::new_string_literal(SPAN, "", None, ast),
            ast,
        )
    } else {
        value
    }
}

pub(super) fn empty<'a>(ast: &AstBuilder<'a>) -> Expression<'a> {
    Expression::new_object_expression(SPAN, oxc_allocator::Vec::new_in(ast), ast)
}

pub(crate) struct Lowering<'b, 'a> {
    pub(crate) ast: &'b AstBuilder<'a>,
    pub(crate) order: Option<u8>,
    pub(crate) filename: Option<&'b str>,
    pub(crate) alternate_order: Option<AlternateOrder>,
}

#[derive(Debug, Clone, Copy)]
pub struct AlternateOrder {
    pub(super) value: Option<u8>,
}

pub(super) struct Classes<'a> {
    pub(super) primary: Expression<'a>,
    pub(super) alternate: Option<Expression<'a>>,
}

impl<'a> Lowering<'_, 'a> {
    pub(crate) fn lower(
        &self,
        source: &Expression<'a>,
        styles: &mut [ExtractStyleProp<'a>],
    ) -> Expression<'a> {
        let ast = self.ast;
        let source = unwrap_syntax_only(source);
        if let [
            ExtractStyleProp::StaticArray(inner)
            | ExtractStyleProp::Evaluated { styles: inner, .. },
        ] = styles
        {
            return self.lower(source, inner);
        }
        match (source, styles) {
            (
                Expression::ConditionalExpression(source),
                [
                    ExtractStyleProp::Conditional {
                        consequent,
                        alternate,
                        ..
                    },
                ],
            ) => Expression::new_conditional_expression(
                SPAN,
                source.test.clone_in(ast.allocator()),
                self.branch(&source.consequent, consequent),
                self.branch(&source.alternate, alternate),
                ast,
            ),
            (
                Expression::LogicalExpression(source),
                [
                    ExtractStyleProp::Conditional {
                        consequent,
                        alternate,
                        ..
                    },
                ],
            ) => {
                let (left, right) = match source.operator {
                    LogicalOperator::And => (alternate, consequent),
                    LogicalOperator::Or | LogicalOperator::Coalesce => (consequent, alternate),
                };
                let selected = if source.operator == LogicalOperator::And {
                    self.leaf(
                        &source.left,
                        left.as_ref()
                            .map_or(&[], |style| std::slice::from_ref(style.as_ref())),
                    )
                } else {
                    self.branch(&source.left, left)
                };
                let value = projection(ast, "__devupLeft", 2);
                let test = match source.operator {
                    LogicalOperator::And => Expression::new_unary_expression(
                        SPAN,
                        UnaryOperator::LogicalNot,
                        value,
                        ast,
                    ),
                    LogicalOperator::Or => value,
                    LogicalOperator::Coalesce => Expression::new_logical_expression(
                        SPAN,
                        Expression::new_binary_expression(
                            SPAN,
                            value.clone_in(ast.allocator()),
                            BinaryOperator::StrictInequality,
                            Expression::new_null_literal(SPAN, ast),
                            ast,
                        ),
                        LogicalOperator::And,
                        Expression::new_binary_expression(
                            SPAN,
                            value,
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
                let left = call_with_values(
                    ast,
                    vec![("__devupLeft".to_string(), selected)],
                    Expression::new_conditional_expression(
                        SPAN,
                        test,
                        Expression::new_identifier(SPAN, "__devupLeft", ast),
                        Expression::new_null_literal(SPAN, ast),
                        ast,
                    ),
                );
                Expression::new_logical_expression(
                    SPAN,
                    left,
                    LogicalOperator::Coalesce,
                    self.branch(&source.right, right),
                    ast,
                )
            }
            (Expression::ComputedMemberExpression(source), styles) => self.member(source, styles),
            (Expression::ArrayExpression(source), styles) => self.array(source, styles),
            (Expression::ObjectExpression(source), styles) => self.object(source, styles),
            (_, styles) => self.leaf(source, styles),
        }
    }

    fn branch(
        &self,
        source: &Expression<'a>,
        styles: &mut Option<Box<ExtractStyleProp<'a>>>,
    ) -> Expression<'a> {
        self.lower(
            source,
            styles
                .as_mut()
                .map_or(&mut [], |style| std::slice::from_mut(style.as_mut())),
        )
    }
}
