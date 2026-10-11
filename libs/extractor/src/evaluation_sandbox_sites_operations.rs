//! Original operation positions for proxy traps without explicit member lookups.

use oxc_ast::ast::{
    BinaryExpression, ForInStatement, SpreadElement, TemplateLiteral, UnaryExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;

pub(super) struct Operations<'r, 's>(pub(super) &'r mut super::Reads<'s>);

impl super::Reads<'_> {
    fn value_site(&mut self, value: Span, operation: Span) {
        let id = self.site(operation);
        self.changes.push((
            Span::new(value.start, value.start),
            format!("{}({id}, ", self.helper),
        ));
        self.changes
            .push((Span::new(value.end, value.end), ")".to_string()));
    }
}

impl<'a> Visit<'a> for Operations<'_, '_> {
    fn visit_binary_expression(&mut self, binary: &BinaryExpression<'a>) {
        self.0.value_site(binary.right.span(), binary.span);
        walk::walk_binary_expression(self, binary);
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression<'a>) {
        match unary.operator {
            UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation | UnaryOperator::BitwiseNot => {
                self.0.value_site(unary.argument.span(), unary.span);
            }
            UnaryOperator::LogicalNot
            | UnaryOperator::Typeof
            | UnaryOperator::Void
            | UnaryOperator::Delete => {}
        }
        walk::walk_unary_expression(self, unary);
    }

    fn visit_spread_element(&mut self, spread: &SpreadElement<'a>) {
        self.0.value_site(spread.argument.span(), spread.span);
        walk::walk_spread_element(self, spread);
    }

    fn visit_for_in_statement(&mut self, statement: &ForInStatement<'a>) {
        self.0.value_site(statement.right.span(), statement.span);
        walk::walk_for_in_statement(self, statement);
    }

    fn visit_template_literal(&mut self, template: &TemplateLiteral<'a>) {
        for expression in &template.expressions {
            self.0.value_site(expression.span(), expression.span());
        }
        walk::walk_template_literal(self, template);
    }
}
