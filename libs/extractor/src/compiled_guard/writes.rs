use oxc_allocator::GetAddress;
use oxc_ast::{
    AstKind,
    ast::{
        AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty, Expression,
        SimpleAssignmentTarget,
    },
};
use oxc_ast_visit::Visit;

use super::{
    callable::{Callables, Proof},
    scopes::Scopes,
};

pub(super) struct Writes<'s> {
    pub scopes: &'s Scopes,
    pub callables: &'s mut Callables,
}

impl Writes<'_> {
    fn name(&mut self, scope: usize, name: &str) {
        if self.scopes.owner(scope, name) == Some(0) {
            self.callables.locals.remove(name);
        }
    }

    fn target(&mut self, target: &AssignmentTarget<'_>, scope: usize) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(id) => self.name(scope, id.name.as_str()),
            AssignmentTarget::ArrayAssignmentTarget(array) => {
                for element in array.elements.iter().flatten() {
                    self.default_target(element, scope);
                }
                if let Some(rest) = &array.rest {
                    self.target(&rest.target, scope);
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(object) => {
                for property in &object.properties {
                    match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => {
                            self.name(scope, property.binding.name.as_str());
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                            self.default_target(&property.binding, scope);
                        }
                    }
                }
                if let Some(rest) = &object.rest {
                    self.target(&rest.target, scope);
                }
            }
            _ => {}
        }
    }

    fn default_target(&mut self, target: &AssignmentTargetMaybeDefault<'_>, scope: usize) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(default) => {
                self.target(&default.binding, scope);
            }
            _ => {
                if let Some(target) = target.as_assignment_target() {
                    self.target(target, scope);
                }
            }
        }
    }
}

impl<'a> Visit<'a> for Writes<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let scope = self.scopes.at(kind.address());
        match kind {
            AstKind::AssignmentExpression(node) => self.target(&node.left, scope),
            AstKind::UpdateExpression(node) => {
                if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &node.argument {
                    self.name(scope, id.name.as_str());
                }
            }
            AstKind::ForOfStatement(node) => {
                if let Some(target) = node.left.as_assignment_target() {
                    self.target(target, scope);
                }
            }
            AstKind::ForInStatement(node) => {
                if let Some(target) = node.left.as_assignment_target() {
                    self.target(target, scope);
                }
            }
            AstKind::CallExpression(node) => {
                if let Expression::Identifier(callee) = &node.callee
                    && callee.name == "eval"
                    && !node.optional
                    && self.scopes.owner(scope, "eval").is_none()
                {
                    self.callables.locals.retain(|name, proof| {
                        !matches!(proof, Proof::Declaration(_, _))
                            || self.scopes.owner(scope, name) != Some(0)
                    });
                }
            }
            _ => {}
        }
    }
}
