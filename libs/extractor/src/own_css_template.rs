use super::{BoundRules, RulePart};
use crate::{
    ExtractStyleProp,
    css_utils::{TemplateStyles, css_to_style_template},
    gen_class_name::{gen_class_names, merge_expression_for_class_name},
    own_css_plan::Node,
    utils::unplaced_error,
    visit::DevupVisitor,
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{Expression, Str};
use oxc_span::SPAN;

impl<'a> DevupVisitor<'a> {
    pub(super) fn own_template(
        &mut self,
        rules: &mut BoundRules<'a>,
        offset: u32,
    ) -> Expression<'a> {
        let quasis = rules
            .plan
            .template
            .iter()
            .flatten()
            .enumerate()
            .map(|(index, raw)| {
                oxc_ast::ast::TemplateElement::new(
                    SPAN,
                    oxc_ast::ast::TemplateElementValue {
                        raw: Str::from_in(raw, self.ast.allocator()),
                        cooked: None,
                    },
                    index == rules.plan.roots.len(),
                    &self.ast,
                )
            });
        let expressions = rules
            .plan
            .roots
            .iter()
            .map(|root| match rules.plan.nodes[*root] {
                Node::Scalar(slot) => {
                    rules.original[slot].clone_in_with_semantic_ids(self.ast.allocator())
                }
                Node::Array(_) | Node::Object(_) => Expression::new_object_expression(
                    SPAN,
                    oxc_allocator::Vec::new_in(&self.ast),
                    &self.ast,
                ),
            });
        let mut template = oxc_ast::ast::TemplateLiteral::new(
            SPAN,
            oxc_allocator::Vec::from_iter_in(quasis, &self.ast),
            oxc_allocator::Vec::from_iter_in(expressions, &self.ast),
            &self.ast,
        );
        self.style_values.read_in_text(&self.ast, &mut template);
        self.own_errors(offset, &crate::dead_properties::template_errors(&template));
        let TemplateStyles {
            styles,
            statements,
            unplaced,
        } = css_to_style_template(&template, 0, &None);
        let mut props: Vec<_> = styles
            .into_iter()
            .map(|s| ExtractStyleProp::Static(s.into()))
            .collect();
        self.own_errors(offset, &props);
        for index in unplaced {
            self.errors
                .push((offset, unplaced_error(&template.expressions[index])));
        }
        let class = gen_class_names(&self.ast, &mut props, None, self.split_filename.as_deref());
        self.styles
            .extend(props.into_iter().flat_map(ExtractStyleProp::into_extract));
        let mut mixins = Vec::new();
        for index in statements {
            let root = rules.plan.roots[index];
            let part = match &rules.plan.nodes[root] {
                Node::Scalar(slot) => {
                    let mut value =
                        rules.original[*slot].clone_in_with_semantic_ids(self.ast.allocator());
                    self.style_values.read_in(&self.ast, &mut value);
                    mixins.push(value);
                    continue;
                }
                Node::Array(items) => RulePart {
                    roots: items.clone(),
                    composing: true,
                },
                Node::Object(_) => RulePart {
                    roots: vec![root],
                    composing: false,
                },
            };
            mixins.push(self.own_class(rules, part, offset));
        }
        self.css_styles = None;
        merge_expression_for_class_name(&self.ast, mixins.into_iter().chain(class))
            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast))
    }
}
