use super::{
    AstKind, Binding, GetSpan, NodeId, Normalizer, REQUIRE_REQUIREMENT, Root, macro_name,
    runtime_name,
};

impl Normalizer<'_, '_> {
    pub(super) fn lower_reads(&mut self) {
        let mut bindings: Vec<_> = self
            .bindings
            .iter()
            .map(|(symbol, binding)| (*symbol, binding.clone()))
            .collect();
        bindings.sort_by_key(|(symbol, _)| *symbol);
        for (symbol, binding) in bindings {
            for reference_id in self.semantic.scoping().get_resolved_reference_ids(symbol) {
                let reference = self.semantic.scoping().get_reference(*reference_id);
                if reference.is_type() {
                    continue;
                }
                if !matches!(binding, Binding::String(_)) {
                    self.check_order(*reference_id);
                }
                let node = self.expression_node(reference.node_id());
                let span = self.semantic.nodes().kind(node).span();
                if self.aliases.contains(&span) {
                    continue;
                }
                match &binding {
                    Binding::Namespace(root) => self.read_namespace(*root, node),
                    Binding::Macro(api) => self.lower_macro(node, api),
                    Binding::String(_) => {}
                }
            }
        }
        for node in self.semantic.nodes().iter() {
            let root = match node.kind() {
                AstKind::CallExpression(call) => self.require_call(call),
                AstKind::AwaitExpression(wait) => self.awaited_import(wait),
                _ => None,
            };
            if let Some(root) = root {
                let node = self.expression_node(node.id());
                let span = self.semantic.nodes().kind(node).span();
                if !self.aliases.contains(&span) {
                    self.read_namespace(root, node);
                }
            }
        }
    }

    fn read_namespace(&mut self, root: Root, node: NodeId) {
        if !self.lower_member(root, node) {
            let span = self.semantic.nodes().kind(node).span();
            self.error(
                span,
                "a namespace may only be read through statically known members or const aliases",
            );
        }
    }

    /// Lower the member read of a namespace expression `node`; `false` when
    /// `node` is not the object of a member read at all.
    fn lower_member(&mut self, root: Root, node: NodeId) -> bool {
        let semantic = self.semantic;
        let span = semantic.nodes().kind(node).span();
        let parent = semantic.nodes().parent_id(node);
        let (key, optional, member_span) = match semantic.nodes().kind(parent) {
            AstKind::StaticMemberExpression(member) if member.object.span() == span => (
                Some(member.property.name.to_string()),
                member.optional,
                member.span,
            ),
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                self.check_read(&member.expression);
                (
                    self.string(&member.expression),
                    member.optional,
                    member.span,
                )
            }
            _ => return false,
        };
        let member = self.expression_node(parent);
        if self.aliases.contains(&semantic.nodes().kind(member).span()) {
            return true;
        }
        let Some(key) = key else {
            self.error(
                member_span,
                "namespace member keys must be constant strings",
            );
            return true;
        };
        if let Some(api) = macro_name(&key) {
            if optional {
                self.error(member_span, "optional namespace reads cannot be compiled");
            } else if matches!(root, Root::Require(_)) {
                self.error(member_span, REQUIRE_REQUIREMENT);
            } else {
                self.lower_macro(parent, api);
            }
        } else if runtime_name(&key) {
            self.runtime.insert(root);
        } else {
            self.error(
                member_span,
                "use a supported styling API or a runtime-only Emotion export",
            );
        }
        true
    }

    fn lower_macro(&mut self, node: NodeId, api: &'static str) {
        let expression = self.expression_node(node);
        let span = self.semantic.nodes().kind(expression).span();
        let callable = match self.semantic.nodes().parent_kind(expression) {
            AstKind::CallExpression(call) => call.callee.span() == span && !call.optional,
            AstKind::TaggedTemplateExpression(tagged) => tagged.tag.span() == span,
            _ => false,
        };
        if callable {
            let name = self.fresh(api);
            self.replace(span, name);
        } else {
            self.error(span, "styling functions must be called directly, not mutated, exported or passed as values");
        }
    }
}

#[cfg(test)]
mod coverage {
    include!("../emotion_namespace_tests/coverage.rs");
}
