use super::Apis;
use crate::barrel::native::Shape;
use crate::utils::unwrap_syntax_only;
use oxc_ast::{
    AstKind,
    ast::{BindingPattern, Expression, ObjectPropertyKind},
};
use std::{collections::BTreeMap, rc::Rc};

impl Apis<'_, '_> {
    pub(in crate::ordinary_ve::selection) fn key(
        &self,
        expression: &Expression<'_>,
    ) -> Option<String> {
        crate::ordinary_ve::selection::static_key::resolve(expression, self.semantic)
    }

    pub(super) fn follow_aliases(&mut self) {
        let semantic = self.semantic;
        loop {
            let before = self.shapes.len();
            for node in semantic.nodes().iter() {
                let AstKind::VariableDeclarator(declarator) = node.kind() else {
                    continue;
                };
                let AstKind::VariableDeclaration(declaration) =
                    semantic.nodes().parent_kind(node.id())
                else {
                    continue;
                };
                if declaration.declare {
                    continue;
                }
                let Some(init) = &declarator.init else {
                    continue;
                };
                let shape = self.shape(init).or_else(|| {
                    let Expression::ObjectExpression(object) = unwrap_syntax_only(init) else {
                        return None;
                    };
                    let members: BTreeMap<_, _> = object.properties.iter().try_fold(
                        BTreeMap::new(),
                        |mut members, property| {
                            let ObjectPropertyKind::ObjectProperty(property) = property else {
                                return None;
                            };
                            let key = if property.computed {
                                property.key.as_expression().and_then(|key| self.key(key))
                            } else {
                                property.key.static_name().map(std::borrow::Cow::into_owned)
                            }?;
                            match self.shape(&property.value) {
                                Some(shape) => {
                                    members.insert(key, shape);
                                }
                                None => {
                                    members.remove(&key);
                                }
                            }
                            Some(members)
                        },
                    )?;
                    (!members.is_empty()).then(|| Rc::new(Shape::Namespace(members)))
                });
                if let Some(shape) = shape {
                    self.alias(&declarator.id, shape);
                }
            }
            if self.shapes.len() == before {
                break;
            }
        }
    }

    fn alias(&mut self, pattern: &BindingPattern<'_>, shape: Rc<Shape>) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                if let Some(symbol) = id.symbol_id.get() {
                    if let Some(binding) = Self::native(&shape) {
                        self.bindings.entry(symbol).or_insert(binding);
                    }
                    self.shapes.entry(symbol).or_insert(shape);
                }
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    let key = if property.computed {
                        property.key.as_expression().and_then(|key| self.key(key))
                    } else {
                        property.key.static_name().map(std::borrow::Cow::into_owned)
                    };
                    if let Some(member) = key.as_deref().and_then(|key| shape.member(key)) {
                        self.alias(&property.value, member);
                    }
                }
            }
            BindingPattern::AssignmentPattern(pattern) => self.alias(&pattern.left, shape),
            BindingPattern::ArrayPattern(_) => {}
        }
    }
}
