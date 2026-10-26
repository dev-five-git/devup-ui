use std::rc::Rc;

use oxc_ast::ast::{
    Expression, IdentifierReference, JSXElementName, JSXMemberExpressionObject, ObjectExpression,
};
use oxc_ast_visit::Visit;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Bindings, NAMESPACE_STYLED, visits};
use crate::component::ExportVariableKind;
use crate::stylex::StylexFunction;
use crate::util_type::UtilType;

impl Bindings {
    /// The export of the package `expression` reads as a member of the
    /// package imported whole
    fn member<'e>(&self, expression: &'e Expression<'_>) -> Option<&'e str> {
        self.member_of(&self.namespaces, expression)
    }

    fn member_of<'e>(
        &self,
        namespaces: &FxHashSet<SymbolId>,
        expression: &'e Expression<'_>,
    ) -> Option<&'e str> {
        if let Expression::StaticMemberExpression(member) = expression
            && let Expression::Identifier(object) = &member.object
            && self
                .symbol(object)
                .is_some_and(|symbol| namespaces.contains(&symbol))
        {
            Some(member.property.name.as_str())
        } else {
            None
        }
    }

    /// The component `expression` reads
    pub fn kind(&self, expression: &Expression<'_>) -> Option<ExportVariableKind> {
        match expression {
            Expression::Identifier(identifier) => self
                .symbol(identifier)
                .and_then(|symbol| self.imports.get(&symbol))
                .cloned(),
            expression => self.member(expression).and_then(|name| name.parse().ok()),
        }
    }

    /// The component the element `name` renders
    pub fn element(&self, name: &JSXElementName<'_>) -> Option<ExportVariableKind> {
        match name {
            JSXElementName::IdentifierReference(identifier) => self
                .symbol(identifier)
                .and_then(|symbol| self.imports.get(&symbol))
                .cloned(),
            JSXElementName::MemberExpression(member) => self
                .namespace_object(&member.object)
                .then(|| member.property.name.as_str().parse().ok())
                .flatten(),
            _ => None,
        }
    }

    fn namespace_object(&self, object: &JSXMemberExpressionObject<'_>) -> bool {
        matches!(object, JSXMemberExpressionObject::IdentifierReference(identifier)
            if self.is_namespace(identifier))
    }

    /// Whether `identifier` reads the package imported whole
    pub fn is_namespace(&self, identifier: &IdentifierReference<'_>) -> bool {
        self.symbol(identifier)
            .is_some_and(|symbol| self.namespaces.contains(&symbol))
    }

    /// Whether the element `name` is the `Global` component
    pub fn is_global_component(&self, name: &JSXElementName<'_>) -> bool {
        match name {
            JSXElementName::IdentifierReference(identifier) => self
                .symbol(identifier)
                .is_some_and(|symbol| self.global_components.contains(&symbol)),
            JSXElementName::MemberExpression(member) => {
                self.namespace_object(&member.object) && member.property.name == "Global"
            }
            _ => false,
        }
    }

    /// Whether `name` is the `ClassNames` component
    pub fn is_class_names(&self, name: &IdentifierReference<'_>) -> bool {
        self.symbol(name)
            .is_some_and(|symbol| self.class_names_components.contains(&symbol))
    }

    /// The style function `callee` reads: `css`, `Devup.keyframes`, ...
    pub fn util(&self, callee: &Expression<'_>) -> Option<Rc<UtilType>> {
        match callee {
            Expression::Identifier(identifier) => self
                .symbol(identifier)
                .and_then(|symbol| self.util_imports.get(&symbol))
                .cloned(),
            callee => self
                .member(callee)
                .and_then(UtilType::from_str_opt)
                .map(Rc::new),
        }
    }

    /// Whether `expression` reads `styled`
    pub fn is_styled(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::Identifier(identifier) => {
                identifier.name == NAMESPACE_STYLED
                    || self
                        .symbol(identifier)
                        .is_some_and(|symbol| self.styled_imports.contains(&symbol))
            }
            expression => self.member(expression) == Some("styled"),
        }
    }

    /// Whether `callee` is a style function or a supported styled factory
    pub fn compiles(&self, callee: &Expression<'_>) -> bool {
        self.util(callee).is_some() || self.is_styled(callee) || self.styled_factory(callee)
    }

    fn styled_factory(&self, callee: &Expression<'_>) -> bool {
        match callee {
            Expression::CallExpression(call) => {
                self.is_styled(&call.callee)
                    || matches!(&call.callee, Expression::StaticMemberExpression(member)
                        if matches!(member.property.name.as_str(), "attrs" | "withConfig")
                            && matches!(call.arguments.as_slice(), [argument] if argument.is_expression())
                            && self.styled_factory(&member.object))
            }
            Expression::StaticMemberExpression(member) => self.is_styled(&member.object),
            _ => false,
        }
    }

    /// Whether `callee` reads the imported `styled`
    pub fn styles(&self, callee: &Expression<'_>) -> bool {
        self.is_styled(callee)
    }

    /// Whether `identifier` reads a component the build renders in its place
    pub fn renders(&self, identifier: &IdentifierReference<'_>) -> bool {
        self.symbol(identifier).is_some_and(|symbol| {
            self.imports.contains_key(&symbol)
                || self.global_components.contains(&symbol)
                || self.class_names_components.contains(&symbol)
        })
    }

    /// Whether the file may read `styled`
    pub fn may_style(&self) -> bool {
        !self.styled_imports.is_empty() || !self.namespaces.is_empty()
    }

    /// The function building elements `callee` reads
    pub fn jsx_function(&self, callee: &Expression<'_>) -> Option<String> {
        match callee {
            Expression::Identifier(identifier) => self
                .symbol(identifier)
                .and_then(|symbol| self.jsx_imports.get(&symbol))
                .cloned(),
            callee => self
                .member_of(&self.jsx_namespaces, callee)
                .map(str::to_string),
        }
    }

    /// The `StyleX` API `callee` reads, as a named import or as a member of
    /// the package imported whole
    pub fn stylex_function(&self, callee: &Expression<'_>) -> Option<StylexFunction> {
        match callee {
            Expression::Identifier(identifier) => self
                .symbol(identifier)
                .and_then(|symbol| self.stylex_imports.get(&symbol))
                .cloned(),
            callee => self
                .member_of(&self.stylex_namespaces, callee)
                .and_then(StylexFunction::from_export_name),
        }
    }

    /// The bindings the names `object` reads all stand for, by name
    pub fn visible(&self, object: &ObjectExpression<'_>) -> FxHashMap<String, SymbolId> {
        let mut reads = visits::Names::new(self);
        reads.visit_object_expression(object);
        reads.into_visible()
    }
}
