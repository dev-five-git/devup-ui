use super::{
    AstKind, Binding, GetSpan, NodeId, Normalizer, REQUIRE_REQUIREMENT, Root, macro_name,
    runtime_name,
};
use oxc_ast::ast::{BindingPattern, Expression, ObjectPattern, VariableDeclarator};

/// A scope-independent stand-in: `undefined` can be shadowed, `void 0` cannot.
const ELIMINATED: &str = "void 0";

impl Normalizer<'_, '_> {
    pub(super) fn lower_bindings(&mut self) {
        for node in self.semantic.nodes().iter() {
            let AstKind::VariableDeclarator(declarator) = node.kind() else {
                continue;
            };
            let Some(init) = &declarator.init else {
                continue;
            };
            let root = match self.value(init) {
                Some(Binding::Macro(_)) => None,
                Some(Binding::Namespace(root)) => Some(root),
                Some(Binding::String(_)) | None => continue,
            };
            if !self.alias_allowed(node.id(), declarator, init) {
                continue;
            }
            if let Expression::ComputedMemberExpression(member) = init.get_inner_expression() {
                self.check_read(&member.expression);
            }
            match root {
                None => self.replace(init.span(), ELIMINATED.into()),
                Some(root) => self.lower_namespace(declarator, init, root),
            }
        }
    }

    fn alias_allowed(
        &mut self,
        node: NodeId,
        declarator: &VariableDeclarator<'_>,
        init: &Expression<'_>,
    ) -> bool {
        if !self.aliases.contains(&init.span()) {
            self.error(
                declarator.span,
                "namespace and styling aliases must be simple const bindings",
            );
            return false;
        }
        let declaration = self.semantic.nodes().parent_id(node);
        if matches!(
            self.semantic.nodes().parent_kind(declaration),
            AstKind::ExportNamedDeclaration(_) | AstKind::ExportDeclaration(_)
        ) {
            self.error(
                declarator.span,
                "namespace and styling function bindings cannot be exported",
            );
            return false;
        }
        true
    }

    fn lower_namespace(
        &mut self,
        declarator: &VariableDeclarator<'_>,
        init: &Expression<'_>,
        root: Root,
    ) {
        let BindingPattern::ObjectPattern(pattern) = &declarator.id else {
            if !self.is_runtime(root) {
                self.replace(init.span(), "{}".into());
            }
            return;
        };
        let fields = self.pattern_fields(pattern, init, root);
        if !matches!(root, Root::Require(_)) {
            self.replace(init.span(), format!("{{{}}}", fields.join(",")));
        }
    }

    /// The members a destructuring keeps reading from the namespace: macros are
    /// eliminated and runtime-only members are copied, so their nested patterns
    /// and defaults keep working.
    fn pattern_fields(
        &mut self,
        pattern: &ObjectPattern<'_>,
        init: &Expression<'_>,
        root: Root,
    ) -> Vec<String> {
        if pattern.rest.is_some() {
            self.error(
                pattern.span,
                "namespace rest destructuring can expose styling functions",
            );
        }
        let mut fields = Vec::new();
        for property in &pattern.properties {
            let Some(key) = self.property_key(property) else {
                self.error(property.span, "destructuring keys must be constant strings");
                continue;
            };
            if property.computed
                && let Some(expression) = property.key.as_expression()
            {
                self.check_read(expression);
            }
            let value = if macro_name(&key).is_some() {
                if !matches!(property.value, BindingPattern::BindingIdentifier(_)) {
                    self.error(
                        property.span,
                        "use a plain binding without defaults or nested patterns",
                    );
                    continue;
                }
                if matches!(root, Root::Require(_)) {
                    self.error(property.span, REQUIRE_REQUIREMENT);
                    continue;
                }
                ELIMINATED.to_string()
            } else {
                if !runtime_name(&key) {
                    self.error(
                        property.span,
                        "use a supported styling API or a runtime-only Emotion export",
                    );
                    continue;
                }
                self.runtime.insert(root);
                format!(
                    "{}[{}]",
                    self.text(init.span()),
                    crate::vanilla_extract::json_string(&key)
                )
            };
            fields.push(format!(
                "{}:{value}",
                crate::vanilla_extract::json_string(&key)
            ));
        }
        fields
    }
}
