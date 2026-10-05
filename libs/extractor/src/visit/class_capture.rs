use super::DevupVisitor;
use super::capture::Captured;
use super::order::{Reach, reach};
use crate::utils::unwrap_syntax_only_mut;
use oxc_allocator::{CloneIn, GetAllocator, TakeIn};
use oxc_ast::ast::{Expression, TemplateElement, TemplateElementValue};
use oxc_span::{GetSpan, SPAN};

impl<'a> DevupVisitor<'a> {
    pub(super) fn capture_class_name(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
        style_order: Option<u8>,
    ) {
        let (styles, replacement) = crate::prop_modify_utils::extract_tailwind_from_class_name(
            &self.ast,
            &Some(value.clone_in_with_semantic_ids(self.ast.allocator())),
            style_order,
            self.split_filename.as_deref(),
        );
        self.styles.extend(styles);
        if let Some(replacement) = replacement {
            *value = replacement;
        }
        if let Expression::TemplateLiteral(template) = unwrap_syntax_only_mut(value) {
            for expression in &mut template.expressions {
                let static_branches = matches!(crate::utils::unwrap_syntax_only(expression),Expression::ConditionalExpression(conditional) if primitive_branch(&conditional.consequent) && primitive_branch(&conditional.alternate));
                if static_branches {
                    self.capture_shape(expression, captured);
                } else if reach(&self.bindings, expression) != Reach::Constant {
                    let span = expression.span();
                    let original = expression.take_in(&self.ast);
                    let quasis = [false, true].into_iter().map(|tail| {
                        TemplateElement::new(
                            SPAN,
                            TemplateElementValue {
                                raw: "".into(),
                                cooked: Some("".into()),
                            },
                            tail,
                            &self.ast,
                        )
                    });
                    *expression = Expression::new_template_literal(
                        span,
                        oxc_allocator::Vec::from_iter_in(quasis, &self.ast),
                        oxc_allocator::Vec::from_array_in([original], &self.ast),
                        &self.ast,
                    );
                    let name = self.names.fresh("__devupValue");
                    captured.push(self.capture_as(name, expression));
                }
            }
        } else {
            self.capture_shape(value, captured);
        }
    }
}

fn primitive_branch(value: &Expression<'_>) -> bool {
    matches!(
        crate::utils::unwrap_syntax_only(value),
        Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
    )
}
