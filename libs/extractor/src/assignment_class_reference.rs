use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_ast_visit::VisitMut;
use oxc_span::{GetSpan, Span};

use crate::utils::expression_to_code;

pub(super) fn replace<'a>(
    ast: &AstBuilder<'a>,
    source: &Expression<'a>,
    class: &mut Expression<'a>,
) {
    ClassReference {
        ast,
        span: source.span(),
        code: expression_to_code(source),
    }
    .visit_expression(class);
}

struct ClassReference<'b, 'a> {
    ast: &'b AstBuilder<'a>,
    span: Span,
    code: String,
}

impl<'a> VisitMut<'a> for ClassReference<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if expression.span() == self.span && expression_to_code(expression) == self.code {
            *expression = Expression::new_identifier(expression.span(), "__devupValue", self.ast);
        } else {
            oxc_ast_visit::walk_mut::walk_expression(self, expression);
        }
    }
}
