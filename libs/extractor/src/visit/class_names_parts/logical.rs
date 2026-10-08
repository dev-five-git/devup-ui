use super::{
    BinaryOperator, CloneIn, Expression, GetAllocator, LocalClass, LocalKnownPart, LocalKnownSide,
    LocalParts, LocalSource, LogicalOperator, SPAN, Text, UnaryOperator, coalesce_keeps_left,
};

enum ChoiceOperator {
    Or,
    Coalesce,
}

impl<'a, S: LocalSource<'a>> LocalParts<'_, '_, 'a, S> {
    pub(super) fn logical_parts(
        &self,
        logical: &oxc_ast::ast::LogicalExpression<'a>,
        parts: &mut Vec<LocalKnownPart<'a, S::Class>>,
        text: Text,
    ) -> Option<()> {
        let visitor = self.visitor;
        let clone =
            |value: &Expression<'a>| value.clone_in_with_semantic_ids(visitor.ast.allocator());
        let operator = match logical.operator {
            LogicalOperator::And => {
                let mut side = Vec::new();
                self.known_parts_local(&logical.right, &mut side, text)?;
                parts.extend(self.guarded_parts_local(&logical.left, side));
                return Some(());
            }
            LogicalOperator::Or => ChoiceOperator::Or,
            LogicalOperator::Coalesce => ChoiceOperator::Coalesce,
        };
        if let Some(finite) = visitor.style_values.finite(&logical.left) {
            let mut left = Vec::new();
            self.known_parts_local(&logical.left, &mut left, text)?;
            let keeps_left = match operator {
                ChoiceOperator::Coalesce => true,
                ChoiceOperator::Or => finite.results.iter().all(|(text, _)| !text.is_empty()),
            };
            if keeps_left {
                parts.extend(left);
                return Some(());
            }
            parts.extend(self.guarded_parts_local(&logical.left, left));
            let mut right = Vec::new();
            self.known_parts_local(&logical.right, &mut right, text)?;
            let inverse = Expression::new_unary_expression(
                SPAN,
                UnaryOperator::LogicalNot,
                clone(&logical.left),
                &visitor.ast,
            );
            parts.extend(self.guarded_parts_local(&inverse, right));
            return Some(());
        }
        match self.known_side_local(&logical.left, text)? {
            LocalKnownSide::Styles(side) => {
                parts.push(LocalKnownPart::Styles(side));
                Some(())
            }
            LocalKnownSide::Empty if coalesce_keeps_left(logical) => Some(()),
            LocalKnownSide::Empty => self.known_parts_local(&logical.right, parts, text),
            LocalKnownSide::Class(left) => {
                let test = match operator {
                    ChoiceOperator::Or => left.clone_source_expression(visitor.ast.allocator()),
                    ChoiceOperator::Coalesce => Expression::new_binary_expression(
                        SPAN,
                        left.clone_source_expression(visitor.ast.allocator()),
                        BinaryOperator::Inequality,
                        Expression::new_null_literal(SPAN, &visitor.ast),
                        &visitor.ast,
                    ),
                };
                let right = self.known_side_local(&logical.right, text)?;
                self.push_choice_local(
                    parts,
                    &test,
                    (
                        LocalKnownSide::Class(left.string_class(&visitor.ast)),
                        right,
                    ),
                );
                Some(())
            }
        }
    }
}
