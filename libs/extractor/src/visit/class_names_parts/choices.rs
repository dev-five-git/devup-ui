use super::{
    CloneIn, Expression, GetAllocator, LocalKnownPart, LocalKnownSide, LocalParts, LocalSource,
    LogicalOperator, SPAN, StringLiteral, UnaryOperator,
};
use crate::gen_class_name::roots::{ClassConditional, ClassConstructors, ClassPayload};
use oxc_ast::ast::ConditionalExpression;

impl<'a, S: LocalSource<'a>> LocalParts<'_, '_, 'a, S> {
    pub(super) fn push_choice_local(
        &self,
        parts: &mut Vec<LocalKnownPart<'a, S::Source>>,
        test: &Expression<'a>,
        sides: (LocalKnownSide<'a, S::Source>, LocalKnownSide<'a, S::Source>),
    ) {
        let ast = &self.visitor.ast;
        let mut classes = [None, None];
        let mut styles = [None, None];
        for (index, side) in [sides.0, sides.1].into_iter().enumerate() {
            match side {
                LocalKnownSide::Styles(side) => styles[index] = Some(side),
                LocalKnownSide::Class(class) => classes[index] = Some(class),
                LocalKnownSide::Empty => {}
            }
        }
        if classes.iter().any(Option::is_some) {
            let [consequent, alternate] = classes.map(|class| {
                class.unwrap_or_else(|| {
                    S::Source::from_string(StringLiteral::boxed(SPAN, "", None, ast))
                })
            });
            parts.push(LocalKnownPart::Class(S::Source::from_conditional(
                ConditionalExpression::boxed(
                    SPAN,
                    test.clone_in_with_semantic_ids(ast.allocator()),
                    consequent.into_expression(),
                    alternate.into_expression(),
                    ast,
                ),
            )));
        }
        if styles.iter().any(Option::is_some) {
            let [consequent, alternate] = styles.map(Option::unwrap_or_default);
            parts.push(LocalKnownPart::Conditional {
                test: test.clone_in_with_semantic_ids(ast.allocator()),
                consequent,
                alternate,
            });
        }
    }

    pub(in crate::visit) fn guarded_parts_local(
        &self,
        test: &Expression<'a>,
        parts: Vec<LocalKnownPart<'a, S::Source>>,
    ) -> Vec<LocalKnownPart<'a, S::Source>> {
        let ast = &self.visitor.ast;
        let mut guarded = Vec::new();
        for part in parts {
            match part {
                LocalKnownPart::Styles(consequent) => guarded.push(LocalKnownPart::Conditional {
                    test: test.clone_in(ast.allocator()),
                    consequent,
                    alternate: Vec::new(),
                }),
                LocalKnownPart::Class(class) => guarded.push(LocalKnownPart::Class(
                    S::Source::from_conditional(ConditionalExpression::boxed(
                        SPAN,
                        test.clone_in(ast.allocator()),
                        class.into_expression(),
                        Expression::new_string_literal(SPAN, "", None, ast),
                        ast,
                    )),
                )),
                LocalKnownPart::Conditional {
                    test: inner,
                    consequent,
                    alternate,
                } => {
                    let yes = Expression::new_logical_expression(
                        SPAN,
                        test.clone_in(ast.allocator()),
                        LogicalOperator::And,
                        inner.clone_in(ast.allocator()),
                        ast,
                    );
                    let no = Expression::new_logical_expression(
                        SPAN,
                        test.clone_in(ast.allocator()),
                        LogicalOperator::And,
                        Expression::new_unary_expression(
                            SPAN,
                            UnaryOperator::LogicalNot,
                            inner,
                            ast,
                        ),
                        ast,
                    );
                    guarded.push(LocalKnownPart::Conditional {
                        test: yes,
                        consequent,
                        alternate: Vec::new(),
                    });
                    guarded.push(LocalKnownPart::Conditional {
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
