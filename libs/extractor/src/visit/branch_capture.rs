//! Captured branch values retain the static shape of each lazy alternative.

use super::DevupVisitor;
use super::capture::Captured;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{Expression, Str};
use oxc_ast_visit::VisitMut;
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use rustc_hash::FxHashMap;

pub(super) struct BranchReads<'a, 'b> {
    pub ast: &'b oxc_ast::builder::AstBuilder<'a>,
    pub reads: FxHashMap<String, Expression<'a>>,
}

impl<'a> VisitMut<'a> for BranchReads<'a, '_> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        crate::utils::rewrite_expression(expression, &mut |expression| {
            if let Expression::Identifier(identifier) = expression
                && let Some(read) = self.reads.get(identifier.name.as_str())
            {
                *expression = read.clone_in(self.ast.allocator());
                return true;
            }
            false
        });
    }
}

impl<'a> BranchReads<'a, '_> {
    fn rebase_results(
        &mut self,
        rules: &mut [Vec<crate::ExtractStyleProp<'a>>],
        parts: &mut FxHashMap<u32, Vec<crate::composition::KnownPart<'a>>>,
        offsets: &[u32],
        aliases: &mut FxHashMap<String, Vec<crate::ExtractStyleValue>>,
    ) {
        for (name, slot) in &self.reads {
            if let Some(styles) = aliases.get(name).cloned() {
                aliases.insert(crate::utils::readable_code(slot), styles);
            }
        }
        for offset in offsets {
            if let Some(parts) = parts.get_mut(offset) {
                for part in parts {
                    match part {
                        crate::composition::KnownPart::Conditional { test, .. } => {
                            self.visit_expression(test);
                        }
                        crate::composition::KnownPart::Class(class) => self.visit_expression(class),
                        crate::composition::KnownPart::Styles(_) => {}
                    }
                }
            }
        }
        for styles in rules {
            for style in styles {
                let ast = self.ast;
                crate::extractor::styled_record_evaluation::visit_style_expressions(
                    ast,
                    style,
                    &mut |expression| self.visit_expression(expression),
                );
            }
        }
    }
}

impl<'a> DevupVisitor<'a> {
    fn css_controller(&mut self, value: &mut Expression<'a>) -> Expression<'a> {
        let offset = value.span().start;
        let inline = match crate::utils::unwrap_syntax_only(value) {
            Expression::CallExpression(call) => self
                .util_type(&call.callee)
                .is_some_and(|util| util.is_css()),
            Expression::TaggedTemplateExpression(tag) => {
                self.util_type(&tag.tag).is_some_and(|util| util.is_css())
            }
            Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => {
                return Expression::new_boolean_literal(SPAN, true, &self.ast);
            }
            _ => false,
        };
        if !inline {
            return value.clone_in_with_semantic_ids(self.ast.allocator());
        }
        let element = self.css_capture.clone().unwrap_or_default();
        let normalized = self
            .css_value(&element, value.take_in(&self.ast), offset)
            .value;
        let mut parts = Vec::new();
        let Some(()) = self.known_parts(&normalized, &mut parts, super::Text::Rules) else {
            *value = normalized;
            return Expression::new_null_literal(SPAN, &self.ast);
        };
        self.css_cache_collect = true;
        for part in &mut parts {
            let sides = match part {
                crate::composition::KnownPart::Styles(side) => vec![side],
                crate::composition::KnownPart::Conditional {
                    consequent,
                    alternate,
                    ..
                } => vec![consequent, alternate],
                crate::composition::KnownPart::Class(_) => Vec::new(),
            };
            for side in sides {
                let props = self.part_props::<false>(
                    offset,
                    std::mem::take(side),
                    super::PartContext::ComposedCssProp(&element),
                    None,
                );
                let index = self.css_capture_rules.len();
                self.css_capture_rules.push(props);
                side.push(crate::composition::KnownStyles::Cached(index));
            }
        }
        let (control, _) = self.composed_class::<false>(offset, self.copy_known_parts(&parts));
        self.css_cache_collect = false;
        self.inline_css_parts.insert(offset, parts);
        self.css_capture_offsets.push(offset);
        *value = Expression::new_string_literal(oxc_span::Span::empty(offset), "", None, &self.ast);
        control
    }

    pub(super) fn capture_css_logical(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        let value = crate::utils::unwrap_syntax_only_mut(value);
        let owned = (self.css_capture_rules.len(), self.css_capture_offsets.len());
        let Expression::LogicalExpression(mut logical) = value.take_in(&self.ast) else {
            unreachable!()
        };
        let mut left = Vec::new();
        self.capture_shape(&mut logical.left, &mut left);
        let control = self.css_controller(&mut logical.left);
        if left.is_empty()
            && let Some(keep_left) =
                super::capture::literal_choice(&self.bindings, &control, logical.operator)
        {
            *value = if keep_left {
                logical.left.take_in(&self.ast)
            } else {
                logical.right.take_in(&self.ast)
            };
            self.capture_shape(value, captured);
            return;
        }
        let mut right = Vec::new();
        self.capture_shape(&mut logical.right, &mut right);
        let packet = self.names.fresh("__devupBranch");
        let selected = match logical.operator {
            LogicalOperator::And => control,
            LogicalOperator::Or => Expression::new_unary_expression(
                SPAN,
                UnaryOperator::LogicalNot,
                control,
                &self.ast,
            ),
            LogicalOperator::Coalesce => Expression::new_binary_expression(
                SPAN,
                control,
                BinaryOperator::Equality,
                Expression::new_null_literal(SPAN, &self.ast),
                &self.ast,
            ),
        };
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut no = oxc_allocator::Vec::new_in(&self.ast);
        no.push(Expression::new_boolean_literal(SPAN, false, &self.ast).into());
        for (index, (name, _)) in left.iter().enumerate() {
            reads
                .reads
                .insert(name.clone(), self.branch_slot(&packet, index + 1));
            no.push(
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(name.as_str(), self.ast.allocator()),
                    &self.ast,
                )
                .into(),
            );
        }
        let mut yes = no.clone_in(self.ast.allocator());
        yes[0] = Expression::new_boolean_literal(SPAN, true, &self.ast).into();
        for (index, (name, value)) in right.into_iter().enumerate() {
            reads
                .reads
                .insert(name, self.branch_slot(&packet, left.len() + index + 1));
            yes.push(value.into());
        }
        let body = Expression::new_conditional_expression(
            SPAN,
            selected,
            Expression::new_array_expression(SPAN, yes, &self.ast),
            Expression::new_array_expression(SPAN, no, &self.ast),
            &self.ast,
        );
        reads.visit_expression(&mut logical.left);
        reads.visit_expression(&mut logical.right);
        reads.rebase_results(
            &mut self.css_capture_rules[owned.0..],
            &mut self.inline_css_parts,
            &self.css_capture_offsets[owned.1..],
            &mut self.captured_class_styles,
        );
        captured.push((packet.clone(), self.scoped_values(left, body)));
        let left = if logical.operator == LogicalOperator::And {
            Expression::new_boolean_literal(SPAN, false, &self.ast)
        } else {
            logical.left.take_in(&self.ast)
        };
        *value = Expression::new_conditional_expression(
            logical.span,
            self.branch_slot(&packet, 0),
            logical.right.take_in(&self.ast),
            left,
            &self.ast,
        );
    }
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
        captured.push((name, self.scoped_values(vec![(left_name, left)], body)));
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
            let owned = (self.css_capture_rules.len(), self.css_capture_offsets.len());
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
            reads.rebase_results(
                &mut self.css_capture_rules[owned.0..],
                &mut self.inline_css_parts,
                &self.css_capture_offsets[owned.1..],
                &mut self.captured_class_styles,
            );
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

#[cfg(test)]
#[path = "branch_capture_contract_tests.rs"]
mod contract_tests;
