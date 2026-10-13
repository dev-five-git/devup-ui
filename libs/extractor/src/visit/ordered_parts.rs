use super::DevupVisitor;
use crate::composition::KnownPart;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::Expression;
use oxc_span::SPAN;
use oxc_syntax::operator::{LogicalOperator, UnaryOperator};

impl<'a> DevupVisitor<'a> {
    pub(super) fn guarded_parts(
        &self,
        test: &Expression<'a>,
        parts: Vec<KnownPart<'a>>,
    ) -> Vec<KnownPart<'a>> {
        let mut guarded = Vec::new();
        for part in parts {
            match part {
                KnownPart::Styles(consequent) => guarded.push(KnownPart::Conditional {
                    test: test.clone_in(self.ast.allocator()),
                    consequent,
                    alternate: Vec::new(),
                }),
                KnownPart::Class(class) => {
                    guarded.push(KnownPart::Class(Expression::new_conditional_expression(
                        SPAN,
                        test.clone_in(self.ast.allocator()),
                        class,
                        Expression::new_string_literal(SPAN, "", None, &self.ast),
                        &self.ast,
                    )));
                }
                KnownPart::Conditional {
                    test: inner,
                    consequent,
                    alternate,
                } => {
                    let yes = Expression::new_logical_expression(
                        SPAN,
                        test.clone_in(self.ast.allocator()),
                        LogicalOperator::And,
                        inner.clone_in(self.ast.allocator()),
                        &self.ast,
                    );
                    let no = Expression::new_logical_expression(
                        SPAN,
                        test.clone_in(self.ast.allocator()),
                        LogicalOperator::And,
                        Expression::new_unary_expression(
                            SPAN,
                            UnaryOperator::LogicalNot,
                            inner,
                            &self.ast,
                        ),
                        &self.ast,
                    );
                    guarded.push(KnownPart::Conditional {
                        test: yes,
                        consequent,
                        alternate: Vec::new(),
                    });
                    guarded.push(KnownPart::Conditional {
                        test: no,
                        consequent: alternate,
                        alternate: Vec::new(),
                    });
                }
            }
        }
        guarded
    }
}
