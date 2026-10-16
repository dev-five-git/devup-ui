//! Captured branch values retain the static shape of each lazy alternative.

use super::DevupVisitor;
use super::capture::Captured;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{Expression, Str};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use rustc_hash::FxHashMap;

pub(super) struct BranchReads<'a, 'b> {
    pub ast: &'b oxc_ast::builder::AstBuilder<'a>,
    pub reads: FxHashMap<String, Expression<'a>>,
}

impl<'a> VisitMut<'a> for BranchReads<'a, '_> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = expression
            && let Some(read) = self.reads.get(identifier.name.as_str())
        {
            *expression = read.clone_in(self.ast.allocator());
        } else {
            walk_mut::walk_expression(self, expression);
        }
    }
}

impl<'a> DevupVisitor<'a> {
    /// Read one captured branch slot without invoking user code.
    pub(super) fn branch_slot(&self, name: &str, index: usize) -> Expression<'a> {
        Expression::new_computed_member_expression(
            SPAN,
            Expression::new_identifier(SPAN, Str::from_in(name, self.ast.allocator()), &self.ast),
            Expression::new_string_literal(
                SPAN,
                Str::from_in(index.to_string().as_str(), self.ast.allocator()),
                None,
                &self.ast,
            ),
            false,
            &self.ast,
        )
    }

    /// Capture a logical right side only when the original operator evaluates it.
    pub(super) fn capture_logical(
        &mut self,
        logical: &mut oxc_ast::ast::LogicalExpression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        let name = self.names.fresh("__devupBranch");
        let left_name = self.names.fresh("__devupValue");
        let left = logical.left.take_in(&self.ast);
        let left_read = Expression::new_identifier(
            SPAN,
            Str::from_in(left_name.as_str(), self.ast.allocator()),
            &self.ast,
        );
        let mut values = Vec::new();
        self.capture_shape(&mut logical.right, &mut values);
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut elements = oxc_allocator::Vec::new_in(&self.ast);
        elements.push(left_read.clone_in(self.ast.allocator()).into());
        for (index, (leaf, value)) in values.into_iter().enumerate() {
            reads.reads.insert(leaf, self.branch_slot(&name, index + 1));
            elements.push(value.into());
        }
        reads.visit_expression(&mut logical.right);
        logical.left = self.branch_slot(&name, 0);
        let test = match logical.operator {
            LogicalOperator::And => left_read.clone_in(self.ast.allocator()),
            LogicalOperator::Or => Expression::new_unary_expression(
                SPAN,
                UnaryOperator::LogicalNot,
                left_read.clone_in(self.ast.allocator()),
                &self.ast,
            ),
            LogicalOperator::Coalesce => Expression::new_binary_expression(
                SPAN,
                left_read.clone_in(self.ast.allocator()),
                BinaryOperator::Equality,
                Expression::new_null_literal(SPAN, &self.ast),
                &self.ast,
            ),
        };
        let selected = Expression::new_array_expression(SPAN, elements, &self.ast);
        let skipped = Expression::new_array_expression(
            SPAN,
            oxc_allocator::Vec::from_array_in([left_read.into()], &self.ast),
            &self.ast,
        );
        let body = Expression::new_conditional_expression(SPAN, test, selected, skipped, &self.ast);
        captured.push((
            name,
            crate::utils::call_with_values(&self.ast, vec![(left_name, left)], body),
        ));
    }

    /// Capture only the expressions of the branch that is actually selected.
    pub(super) fn capture_conditional(
        &mut self,
        conditional: &mut oxc_ast::ast::ConditionalExpression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        let name = self.names.fresh("__devupBranch");
        let mut branches = Vec::new();
        for (selected, branch) in [
            (true, &mut conditional.consequent),
            (false, &mut conditional.alternate),
        ] {
            let mut values = Vec::new();
            self.capture_shape(branch, &mut values);
            let mut reads = BranchReads {
                ast: &self.ast,
                reads: FxHashMap::default(),
            };
            let mut elements = oxc_allocator::Vec::new_in(&self.ast);
            elements.push(Expression::new_boolean_literal(SPAN, selected, &self.ast).into());
            for (index, (leaf, value)) in values.into_iter().enumerate() {
                reads.reads.insert(leaf, self.branch_slot(&name, index + 1));
                elements.push(value.into());
            }
            reads.visit_expression(branch);
            branches.push(Expression::new_array_expression(SPAN, elements, &self.ast));
        }
        let test = conditional.test.take_in(&self.ast);
        conditional.test = self.branch_slot(&name, 0);
        let mut branches = branches.into_iter();
        if let (Some(consequent), Some(alternate)) = (branches.next(), branches.next()) {
            let values = Expression::new_conditional_expression(
                test.span(),
                test,
                consequent,
                alternate,
                &self.ast,
            );
            captured.push((name, values));
        }
    }
}
