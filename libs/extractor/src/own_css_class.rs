use super::{BoundRules, RulePart};
use crate::{
    ExtractStyleProp,
    composition::{KnownPart, KnownStyles, set_prop_order},
    extractor::ExtractResult,
    gen_class_name::{gen_class_names, merge_expression_for_class_name},
    own_css_plan::Node,
    utils::unwrap_syntax_only,
    visit::{DevupVisitor, Text},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::Expression;
use oxc_span::SPAN;

impl<'a> DevupVisitor<'a> {
    pub(super) fn own_class(
        &mut self,
        rules: &mut BoundRules<'a>,
        part: RulePart,
        offset: u32,
    ) -> Expression<'a> {
        let known = part.roots.iter().any(|id| match rules.plan.nodes[*id] {
            Node::Scalar(slot) => self.reads_known_styles(&rules.original[slot]),
            Node::Object(_) | Node::Array(_) => false,
        });
        if known {
            let mut parts = Vec::new();
            for &root in &part.roots {
                match rules.plan.nodes[root] {
                    Node::Object(_) | Node::Array(_) => {
                        let ExtractResult {
                            mut styles,
                            style_order,
                            ..
                        } = crate::own_css_lower::RuleCompiler {
                            ast: &self.ast,
                            plan: &rules.plan,
                            slots: &rules.slots,
                        }
                        .lower(root);
                        self.own_errors(offset, &styles);
                        if let Some(order) = style_order {
                            for prop in &mut styles {
                                set_prop_order(prop, order);
                            }
                        }
                        parts.push(KnownPart::Styles(vec![KnownStyles::Known(
                            styles
                                .into_iter()
                                .flat_map(ExtractStyleProp::into_extract)
                                .collect(),
                        )]));
                    }
                    Node::Scalar(slot) => {
                        self.known_parts(&rules.original[slot], &mut parts, Text::Classes);
                    }
                }
            }
            let (result, known) = self.composed_class(offset, parts);
            if let Some(known) = known {
                self.css_styles = Some((offset, known));
            }
            return result;
        }
        let mut classes = Vec::new();
        let result = if !part.composing
            && let [root] = part.roots.as_slice()
        {
            crate::own_css_lower::RuleCompiler {
                ast: &self.ast,
                plan: &rules.plan,
                slots: &rules.slots,
            }
            .lower(*root)
        } else {
            for &root in &part.roots {
                if let Node::Scalar(slot) = rules.plan.nodes[root]
                    && match unwrap_syntax_only(&rules.slots[slot]) {
                        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => true,
                        Expression::Identifier(identifier) => identifier.name != "undefined",
                        _ => false,
                    }
                {
                    classes
                        .push(rules.slots[slot].clone_in_with_semantic_ids(self.ast.allocator()));
                }
            }
            let root = crate::own_css_merge::merge(&mut rules.plan, &part.roots);
            crate::own_css_lower::RuleCompiler {
                ast: &self.ast,
                plan: &rules.plan,
                slots: &rules.slots,
            }
            .lower(root)
        };
        let ExtractResult {
            mut styles,
            style_order,
            ..
        } = result;
        self.own_errors(offset, &styles);
        let mut recorded = crate::composition::Composition::default();
        let mut props: Vec<_> = styles
            .iter()
            .map(|p| p.clone_in(self.ast.allocator()))
            .collect();
        if let Some(order) = style_order {
            for prop in &mut props {
                set_prop_order(prop, order);
            }
        }
        recorded.apply(&self.ast, props);
        let class = gen_class_names(
            &self.ast,
            &mut styles,
            style_order,
            self.split_filename.as_deref(),
        );
        self.styles
            .extend(styles.into_iter().flat_map(ExtractStyleProp::into_extract));
        if classes.is_empty()
            && matches!(class, Some(Expression::StringLiteral(_)))
            && let Some(known) = recorded.unconditional()
        {
            self.css_styles = Some((offset, known));
        }
        merge_expression_for_class_name(&self.ast, classes.into_iter().chain(class))
            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, &self.ast))
    }
}
