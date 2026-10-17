//! Original operation positions for proxy traps without explicit member lookups.

use oxc_ast::ast::{
    BinaryExpression, ForInStatement, SpreadElement, TemplateLiteral, UnaryExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;

pub(super) struct Operations<'r, 's>(pub(super) &'r mut super::Reads<'s>);

pub(crate) fn instrument(source: &str, path: &str) -> super::Instrumented {
    super::instrument_located(source, path, &|_| true)
}

pub(super) fn locate_members(
    program: &oxc_ast::ast::Program<'_>,
    reads: &mut super::Reads<'_>,
    authored: &dyn Fn(usize) -> bool,
) {
    struct Members<'r, 's, 'f>(&'r mut super::Reads<'s>, &'f dyn Fn(usize) -> bool);
    impl Members<'_, '_, '_> {
        fn locate(&mut self, from: u32, to: u32) {
            if !(self.1)(from as usize) {
                for (place, offset) in &mut self.0.sites {
                    if *offset == from {
                        *offset = to;
                        *place = crate::locate(self.0.path, self.0.source, to as usize);
                    }
                }
            }
        }
    }
    impl<'a> Visit<'a> for Members<'_, '_, '_> {
        fn visit_static_member_expression(
            &mut self,
            member: &oxc_ast::ast::StaticMemberExpression<'a>,
        ) {
            self.locate(member.span.start, member.property.span.start);
            walk::walk_static_member_expression(self, member);
        }

        fn visit_computed_member_expression(
            &mut self,
            member: &oxc_ast::ast::ComputedMemberExpression<'a>,
        ) {
            self.locate(member.span.start, member.expression.span().start);
            walk::walk_computed_member_expression(self, member);
        }
    }
    Members(reads, authored).visit_program(program);
}

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
