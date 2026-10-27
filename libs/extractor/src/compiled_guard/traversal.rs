use oxc_allocator::{GetAddress, UnstableAddress};
use oxc_ast::ast::{
    AssignmentExpression, AssignmentTarget, AssignmentTargetProperty, BindingPattern,
    BindingProperty, ComputedMemberExpression, ExportAllDeclaration, ExportDeclaration,
    ExportDefaultDeclaration, ExportFromDeclaration, ExportNamedDeclaration, Expression,
    IdentifierReference, ImportDeclaration, ImportMeta, JSXElementName, NewTarget, ObjectProperty,
    PrivateFieldExpression, PropertyKey, StaticMemberExpression, VariableDeclaration,
    VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};

use super::{
    callable::Callables,
    origins::{literal_name, origin, property_name},
    scopes::Scopes,
    text::Text,
    wire::{Facts, Origin, Positions},
};

pub(super) struct Collector {
    pub scopes: Scopes,
    pub facts: Facts,
    pub callables: Option<Callables>,
    positions: Positions,
}

impl Collector {
    pub fn new(scopes: Scopes, positions: Positions, observe: bool) -> Self {
        Self {
            scopes,
            positions,
            facts: Facts::new(observe),
            callables: observe.then(Callables::default),
        }
    }

    pub fn record(&mut self, value: Option<Origin>, pos: u32) {
        if let Some(value) = value.filter(|value| !value.ids.is_empty()) {
            self.facts
                .references
                .push((value, self.positions.utf16(pos)));
        }
    }

    fn member(&mut self, object: &Expression<'_>, name: Option<Text>, pos: u32) {
        let scope = self.scopes.at(object.address());
        let value = origin(object, &self.scopes, scope);
        self.record(
            value.clone().and_then(|mut value| {
                value.ids.push(name?);
                Some(value)
            }),
            pos,
        );
        if value.is_none() {
            self.visit_expression(object);
        }
    }
}

impl<'a> Visit<'a> for Collector {
    fn visit_import_declaration(&mut self, _node: &ImportDeclaration<'a>) {}

    fn visit_identifier_reference(&mut self, node: &IdentifierReference<'a>) {
        let scope = self.scopes.at(node.unstable_address());
        self.record(
            self.scopes.binding(scope, node.name.as_str()),
            node.span.start,
        );
    }

    fn visit_variable_declaration(&mut self, node: &VariableDeclaration<'a>) {
        if node.kind == VariableDeclarationKind::Const {
            let scope = self.scopes.at(node.unstable_address());
            for declaration in &node.declarations {
                if let (BindingPattern::BindingIdentifier(id), Some(init)) =
                    (&declaration.id, &declaration.init)
                    && let Some(value) = origin(init, &self.scopes, scope)
                    && let Some(owner) = self.scopes.owner(scope, id.name.as_str())
                {
                    self.scopes.entries[owner]
                        .bindings
                        .insert(id.name.to_string(), Some(value));
                }
            }
        }
        walk::walk_variable_declaration(self, node);
    }

    fn visit_variable_declarator(&mut self, node: &VariableDeclarator<'a>) {
        if let (BindingPattern::ObjectPattern(pattern), Some(init)) = (&node.id, &node.init) {
            let scope = self.scopes.at(node.unstable_address());
            if let Some(value) = origin(init, &self.scopes, scope) {
                for property in &pattern.properties {
                    if let Some(name) = property_name(&property.key, property.computed) {
                        let mut value = value.clone();
                        value.ids.push(name);
                        self.record(Some(value), property.span.start);
                    }
                }
            }
        }
        walk::walk_variable_declarator(self, node);
    }

    fn visit_assignment_expression(&mut self, node: &AssignmentExpression<'a>) {
        if let AssignmentTarget::ObjectAssignmentTarget(pattern) = &node.left {
            let scope = self.scopes.at(node.unstable_address());
            if let Some(value) = origin(&node.right, &self.scopes, scope) {
                for property in &pattern.properties {
                    let (name, pos) = match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => (
                            Some(Text::from(property.binding.name.as_str())),
                            property.span.start,
                        ),
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => (
                            property_name(&property.name, property.computed),
                            property.span.start,
                        ),
                    };
                    if let Some(name) = name {
                        let mut value = value.clone();
                        value.ids.push(name);
                        self.record(Some(value), pos);
                    }
                }
            }
        }
        walk::walk_assignment_expression(self, node);
    }

    fn visit_static_member_expression(&mut self, node: &StaticMemberExpression<'a>) {
        self.member(
            &node.object,
            Some(Text::from(node.property.name.as_str())),
            node.span.start,
        );
    }

    fn visit_private_field_expression(&mut self, node: &PrivateFieldExpression<'a>) {
        self.member(
            &node.object,
            Some(Text::from(node.field.name.as_str())),
            node.span.start,
        );
    }

    fn visit_computed_member_expression(&mut self, node: &ComputedMemberExpression<'a>) {
        let name = literal_name(&node.expression);
        let computed = name.is_none();
        self.member(&node.object, name, node.span.start);
        if computed {
            self.visit_expression(&node.expression);
        }
    }

    fn visit_binding_property(&mut self, node: &BindingProperty<'a>) {
        if node.computed
            && let Some(key) = node.key.as_expression()
        {
            self.visit_expression(key);
        }
        self.visit_binding_pattern(&node.value);
    }

    fn visit_property_key(&mut self, node: &PropertyKey<'a>) {
        if let Some(key) = node.as_expression()
            && !matches!(key, Expression::Identifier(_))
        {
            self.visit_expression(key);
        }
    }

    fn visit_jsx_element_name(&mut self, _node: &JSXElementName<'a>) {}

    fn visit_import_meta(&mut self, node: &ImportMeta) {
        let scope = self.scopes.at(node.unstable_address());
        self.record(
            self.scopes.binding(scope, "meta"),
            node.span.end.saturating_sub(4),
        );
    }

    fn visit_new_target(&mut self, node: &NewTarget) {
        let scope = self.scopes.at(node.unstable_address());
        self.record(
            self.scopes.binding(scope, "target"),
            node.span.end.saturating_sub(6),
        );
    }

    fn visit_object_property(&mut self, node: &ObjectProperty<'a>) {
        if let Some(key) = node.key.as_expression()
            && !matches!(key, Expression::Identifier(_))
        {
            self.visit_expression(key);
        }
        self.visit_expression(&node.value);
    }

    fn visit_assignment_target(&mut self, node: &AssignmentTarget<'a>) {
        match node {
            AssignmentTarget::ArrayAssignmentTarget(_)
            | AssignmentTarget::ObjectAssignmentTarget(_) => self.assignment_pattern(node),
            _ => walk::walk_assignment_target(self, node),
        }
    }

    fn visit_export_named_declaration(&mut self, node: &ExportNamedDeclaration<'a>) {
        self.named_export(node);
    }

    fn visit_export_from_declaration(&mut self, node: &ExportFromDeclaration<'a>) {
        self.forward_export(node);
    }

    fn visit_export_declaration(&mut self, node: &ExportDeclaration<'a>) {
        self.declared_export(&node.declaration);
        self.visit_declaration(&node.declaration);
    }

    fn visit_export_all_declaration(&mut self, node: &ExportAllDeclaration<'a>) {
        self.star_export(node);
    }

    fn visit_export_default_declaration(&mut self, node: &ExportDefaultDeclaration<'a>) {
        self.default_export(node);
        walk::walk_export_default_declaration(self, node);
    }
}
