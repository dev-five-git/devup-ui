use crate::utils::{
    call_with_values, get_string_by_literal_expression, unwrap_syntax_only, unwrap_syntax_only_mut,
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Argument, ArrayExpressionElement, BinaryOperator, ComputedMemberExpression, Expression,
        LogicalOperator, Str,
    },
    builder::AstBuilder,
};
use oxc_span::{GetSpan, GetSpanMut};

pub(crate) fn capture<'a>(
    ast: &AstBuilder<'a>,
    arguments: &mut [Argument<'a>],
) -> Vec<(String, Expression<'a>)> {
    let mut captures = Captures {
        ast,
        reads: Vec::new(),
    };
    for argument in arguments {
        if let Some(value) = argument.as_expression_mut() {
            captures.part(value);
        }
    }
    captures.reads
}

struct Captures<'b, 'a> {
    ast: &'b AstBuilder<'a>,
    reads: Vec<(String, Expression<'a>)>,
}

impl<'a> Captures<'_, 'a> {
    fn read(&mut self, value: &mut Expression<'a>) {
        let ast = self.ast;
        if get_string_by_literal_expression(value).is_some() {
            return;
        }
        let name = format!("__devupCompose{}", self.reads.len());
        let reference = Expression::new_identifier(
            value.span(),
            Str::from_in(name.as_str(), ast.allocator()),
            ast,
        );
        self.reads.push((name, std::mem::replace(value, reference)));
    }

    fn part(&mut self, value: &mut Expression<'a>) {
        let ast = self.ast;
        match unwrap_syntax_only_mut(value) {
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    if let Some(value) = element.as_expression_mut() {
                        self.part(value);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                let con = class(&conditional.consequent, ast);
                let alt = class(&conditional.alternate, ast);
                let name = self.choice(
                    &mut conditional.test,
                    con.clone_in(ast.allocator()),
                    alt.clone_in(ast.allocator()),
                );
                if con.is_some() {
                    conditional.consequent = slot(ast, &name, 1);
                }
                if alt.is_some() {
                    conditional.alternate = slot(ast, &name, 1);
                }
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator == LogicalOperator::And
                    || !matches!(
                        unwrap_syntax_only(&logical.left),
                        Expression::ObjectExpression(_)
                    )
                {
                    let right = class(&logical.right, ast);
                    let name = if logical.operator == LogicalOperator::And {
                        self.choice(&mut logical.left, right.clone_in(ast.allocator()), None)
                    } else {
                        let original = logical.left.clone_in(ast.allocator());
                        let controller =
                            Expression::new_identifier(oxc_span::SPAN, "__devupChoice", ast);
                        let test = if logical.operator == LogicalOperator::Coalesce {
                            Expression::new_binary_expression(
                                oxc_span::SPAN,
                                controller.clone_in(ast.allocator()),
                                BinaryOperator::Inequality,
                                Expression::new_null_literal(oxc_span::SPAN, ast),
                                ast,
                            )
                        } else {
                            controller.clone_in(ast.allocator())
                        };
                        let selected = Expression::new_conditional_expression(
                            oxc_span::SPAN,
                            test,
                            controller.clone_in(ast.allocator()),
                            right.clone_in(ast.allocator()).unwrap_or_else(|| {
                                Expression::new_string_literal(oxc_span::SPAN, "", None, ast)
                            }),
                            ast,
                        );
                        self.pair(&mut logical.left, original, selected)
                    };
                    if right.is_some() {
                        logical.right = slot(ast, &name, 1);
                    }
                }
            }
            Expression::Identifier(identifier) if identifier.name == "undefined" => {}
            Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::TemplateLiteral(_) => self.read(value),
            _ => {}
        }
    }

    fn choice(
        &mut self,
        value: &mut Expression<'a>,
        con: Option<Expression<'a>>,
        alt: Option<Expression<'a>>,
    ) -> String {
        let ast = self.ast;
        let original = value.clone_in(ast.allocator());
        let controller = Expression::new_identifier(oxc_span::SPAN, "__devupChoice", ast);
        let selected = Expression::new_conditional_expression(
            oxc_span::SPAN,
            controller.clone_in(ast.allocator()),
            con.unwrap_or_else(|| Expression::new_string_literal(oxc_span::SPAN, "", None, ast)),
            alt.unwrap_or_else(|| Expression::new_string_literal(oxc_span::SPAN, "", None, ast)),
            ast,
        );
        self.pair(value, original, selected)
    }

    fn pair(
        &mut self,
        value: &mut Expression<'a>,
        original: Expression<'a>,
        selected: Expression<'a>,
    ) -> String {
        let ast = self.ast;
        let controller = Expression::new_identifier(oxc_span::SPAN, "__devupChoice", ast);
        let name = format!("__devupCompose{}", self.reads.len());
        let tuple = Expression::new_array_expression(
            oxc_span::SPAN,
            oxc_allocator::Vec::from_array_in(
                [
                    ArrayExpressionElement::from(controller),
                    ArrayExpressionElement::from(selected),
                ],
                ast,
            ),
            ast,
        );
        let result = call_with_values(ast, vec![("__devupChoice".to_string(), original)], tuple);
        self.reads.push((name.clone(), result));
        let span = value.span();
        *value = slot(ast, &name, 0);
        *value.span_mut() = span;
        name
    }
}

fn class<'a>(value: &Expression<'a>, ast: &AstBuilder<'a>) -> Option<Expression<'a>> {
    match unwrap_syntax_only(value) {
        Expression::Identifier(identifier) if identifier.name == "undefined" => None,
        Expression::Identifier(_)
        | Expression::StaticMemberExpression(_)
        | Expression::ComputedMemberExpression(_)
        | Expression::StringLiteral(_)
        | Expression::TemplateLiteral(_) => Some(value.clone_in(ast.allocator())),
        _ => None,
    }
}

fn slot<'a>(ast: &AstBuilder<'a>, name: &str, index: u8) -> Expression<'a> {
    Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
        oxc_span::SPAN,
        Expression::new_identifier(oxc_span::SPAN, Str::from_in(name, ast.allocator()), ast),
        Expression::new_numeric_literal(
            oxc_span::SPAN,
            f64::from(index),
            None,
            oxc_syntax::number::NumberBase::Decimal,
            ast,
        ),
        false,
        ast,
    ))
}
