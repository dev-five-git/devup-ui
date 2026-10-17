use super::BoundRules;
use crate::{
    own_css_plan::{Node, Plan},
    utils::{get_string_by_property_key, unwrap_syntax_only},
    visit::DevupVisitor,
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectPropertyKind, Program, VariableDeclarator};
use oxc_ast_visit::Visit;
use rustc_hash::FxHashMap;

impl<'a> DevupVisitor<'a> {
    pub(crate) fn own_rules(&mut self, named: FxHashMap<String, Plan>, program: &Program<'a>) {
        struct Calls {
            named: FxHashMap<String, Plan>,
            spans: FxHashMap<(u32, u32), Plan>,
        }
        impl<'a> Visit<'a> for Calls {
            fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
                if let Some(name) = it.id.get_binding_identifier()
                    && let Some(plan) = self.named.remove(name.name.as_str())
                    && let Some(Expression::CallExpression(call)) = &it.init
                {
                    self.spans.insert((call.span.start, call.span.end), plan);
                }
            }
        }
        let mut calls = Calls {
            named,
            spans: FxHashMap::default(),
        };
        calls.visit_program(program);
        self.own_rules = calls.spans;
    }

    pub(super) fn bind_own_rules(
        &self,
        mut plan: Plan,
        array: &oxc_ast::ast::ArrayExpression<'a>,
    ) -> BoundRules<'a> {
        let original: Vec<_> = array
            .elements
            .iter()
            .filter_map(|item| {
                item.as_expression()
                    .map(|e| e.clone_in_with_semantic_ids(self.ast.allocator()))
            })
            .collect();
        let mut slots: Vec<_> = original
            .iter()
            .map(|e| e.clone_in_with_semantic_ids(self.ast.allocator()))
            .collect();
        for expression in &mut slots {
            self.style_values.read_in(&self.ast, expression);
        }
        for node in &mut plan.nodes {
            match node {
                Node::Object(properties) => {
                    for property in properties {
                        if let Expression::ObjectExpression(object) =
                            unwrap_syntax_only(&slots[property.slot])
                            && let Some(ObjectPropertyKind::ObjectProperty(key)) =
                                object.properties.first()
                            && let Some(name) = get_string_by_property_key(&key.key)
                        {
                            property.name = name;
                        }
                    }
                }
                Node::Scalar(_) | Node::Array(_) => {}
            }
        }
        BoundRules {
            plan,
            original,
            slots,
        }
    }
}
