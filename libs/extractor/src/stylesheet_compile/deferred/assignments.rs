use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{
    AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty,
    AssignmentTargetPropertyProperty, AssignmentTargetWithDefault, IdentifierName, PropertyKey,
    SimpleAssignmentTarget,
};

use super::visit::Deferred;

impl<'a> Deferred<'a, '_> {
    pub(super) fn rewrite_target(&mut self, target: &mut SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) = target
            && let Some(symbol) = identifier
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id())
            && let Some(name) = self.bindings.get(&symbol)
        {
            *target = SimpleAssignmentTarget::ComputedMemberExpression(
                self.access(identifier.span, &name.clone()),
            );
        }
    }

    pub(super) fn rewrite_property(&mut self, property: &mut AssignmentTargetProperty<'a>) {
        if let AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(identifier) = property
            && let Some(symbol) = identifier
                .binding
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id())
            && let Some(name) = self.bindings.get(&symbol)
        {
            let member = self.access(identifier.binding.span, &name.clone());
            let binding = match &identifier.init {
                Some(init) => AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
                    AssignmentTargetWithDefault::boxed(
                        identifier.span,
                        AssignmentTarget::ComputedMemberExpression(member),
                        init.clone_in(self.ast.allocator()),
                        &self.ast,
                    ),
                ),
                None => AssignmentTargetMaybeDefault::ComputedMemberExpression(member),
            };
            *property = AssignmentTargetProperty::AssignmentTargetPropertyProperty(
                AssignmentTargetPropertyProperty::boxed(
                    identifier.span,
                    PropertyKey::StaticIdentifier(IdentifierName::boxed(
                        identifier.binding.span,
                        identifier.binding.name,
                        &self.ast,
                    )),
                    binding,
                    false,
                    &self.ast,
                ),
            );
        }
    }
}
