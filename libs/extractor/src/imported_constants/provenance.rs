//! Syntactic value proofs, independent of whether the constant evaluator can
//! compute a value. Unknown calls are never executed to obtain these facts.

use oxc_ast::{
    AstKind,
    ast::{
        Argument, ArrayExpressionElement, ArrowFunctionBody, Expression, ObjectPropertyKind,
        PropertyKind, Statement,
    },
};
use oxc_semantic::{AstNodes, Scoping};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::{css_prop::binding_of, utils::unwrap_syntax_only};

#[derive(Clone)]
pub(crate) enum Shape {
    Primitive,
    Function,
    Record(Vec<(String, Self)>),
    Array(Vec<Self>),
    Unknown,
}

impl Shape {
    pub(crate) fn primitive_path(&self, path: &[Option<String>]) -> bool {
        match path.split_first() {
            None => matches!(self, Self::Primitive),
            Some((Some(key), rest)) => match self {
                Self::Record(fields) => fields
                    .iter()
                    .rev()
                    .find(|(name, _)| name == key)
                    .is_none_or(|(_, value)| value.primitive_path(rest)),
                Self::Array(values) => match key.parse::<usize>() {
                    Ok(index) => values
                        .get(index)
                        .is_none_or(|value| value.primitive_path(rest)),
                    Err(_) => key == "length" && rest.is_empty(),
                },
                Self::Primitive => true,
                Self::Function | Self::Unknown => false,
            },
            Some((None, rest)) => match self {
                Self::Record(fields) => fields.iter().all(|(_, value)| value.primitive_path(rest)),
                Self::Array(values) => values.iter().all(|value| value.primitive_path(rest)),
                Self::Primitive => true,
                Self::Function | Self::Unknown => false,
            },
        }
    }

    /// Plain data with no accessor, coercion, serialization or prototype hooks.
    pub(crate) fn plain(&self) -> bool {
        match self {
            Self::Primitive => true,
            Self::Record(fields) => fields.iter().all(|(key, value)| {
                !matches!(
                    key.as_str(),
                    "__proto__" | "toJSON" | "toString" | "valueOf"
                ) && value.plain()
            }),
            Self::Array(values) => values.iter().all(Self::plain),
            Self::Function | Self::Unknown => false,
        }
    }

    pub(crate) fn shallow_primitives(&self) -> bool {
        self.plain() && self.primitive_path(&[None])
    }
}

pub(crate) struct Proof<'s, 'a> {
    pub nodes: &'s AstNodes<'a>,
    pub scoping: &'s Scoping,
}

impl<'a> Proof<'_, 'a> {
    pub(crate) fn expression(&self, expression: &Expression<'a>) -> Shape {
        self.value(expression, &mut FxHashSet::default())
    }

    pub(crate) fn binding(&self, symbol: SymbolId) -> Shape {
        self.symbol(symbol, &mut FxHashSet::default())
    }

    fn symbol(&self, symbol: SymbolId, seen: &mut FxHashSet<SymbolId>) -> Shape {
        if !seen.insert(symbol) {
            return Shape::Unknown;
        }
        let value = match self.nodes.kind(self.scoping.symbol_declaration(symbol)) {
            AstKind::VariableDeclarator(declaration) => declaration
                .init
                .as_ref()
                .map_or(Shape::Unknown, |init| self.value(init, seen)),
            AstKind::Function(_) => Shape::Function,
            _ => Shape::Unknown,
        };
        seen.remove(&symbol);
        value
    }

    fn value(&self, expression: &Expression<'a>, seen: &mut FxHashSet<SymbolId>) -> Shape {
        match unwrap_syntax_only(expression) {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => Shape::Primitive,
            Expression::Identifier(identifier) => binding_of(self.scoping, identifier)
                .map_or(Shape::Unknown, |symbol| self.symbol(symbol, seen)),
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                Shape::Function
            }
            Expression::ObjectExpression(object) => {
                let mut fields = Vec::new();
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            match (property.kind, property.computed, property.key.static_name()) {
                                (PropertyKind::Init, false, Some(key)) => fields
                                    .push((key.to_string(), self.value(&property.value, seen))),
                                _ => return Shape::Unknown,
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            let Shape::Record(spread) = self.value(&spread.argument, seen) else {
                                return Shape::Unknown;
                            };
                            fields.extend(spread);
                        }
                    }
                }
                Shape::Record(fields)
            }
            Expression::ArrayExpression(array) => {
                let mut values = Vec::new();
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            let Shape::Array(spread) = self.value(&spread.argument, seen) else {
                                return Shape::Unknown;
                            };
                            values.extend(spread);
                        }
                        ArrayExpressionElement::Elision(_) => values.push(Shape::Primitive),
                        element => values.push(self.value(element.to_expression(), seen)),
                    }
                }
                Shape::Array(values)
            }
            Expression::StaticMemberExpression(member) => {
                self.member(&member.object, member.property.name.as_str(), seen)
            }
            Expression::ComputedMemberExpression(member) => {
                crate::utils::get_string_by_literal_expression(&member.expression)
                    .map_or(Shape::Unknown, |key| {
                        self.member(&member.object, &key, seen)
                    })
            }
            Expression::CallExpression(call) => {
                if crate::mutations::callees::global(self, &call.callee)
                    == Some(("Object", "freeze"))
                {
                    return call
                        .arguments
                        .first()
                        .and_then(Argument::as_expression)
                        .map_or(Shape::Unknown, |argument| self.value(argument, seen));
                }
                let Expression::Identifier(callee) = &call.callee else {
                    return Shape::Unknown;
                };
                let Some(symbol) = binding_of(self.scoping, callee) else {
                    return Shape::Unknown;
                };
                if !call.arguments.is_empty() || !seen.insert(symbol) {
                    return Shape::Unknown;
                }
                let result = self
                    .factory(symbol)
                    .map_or(Shape::Unknown, |body| self.value(body, seen));
                seen.remove(&symbol);
                result
            }
            _ => Shape::Unknown,
        }
    }

    fn member(&self, object: &Expression<'a>, key: &str, seen: &mut FxHashSet<SymbolId>) -> Shape {
        match self.value(object, seen) {
            Shape::Record(fields) => fields
                .into_iter()
                .rev()
                .find(|(name, _)| name == key)
                .map_or(Shape::Primitive, |(_, value)| value),
            Shape::Array(values) => match key.parse::<usize>() {
                Ok(index) => values.get(index).cloned().unwrap_or(Shape::Primitive),
                Err(_) => {
                    if key == "length" {
                        Shape::Primitive
                    } else {
                        Shape::Unknown
                    }
                }
            },
            Shape::Primitive | Shape::Function | Shape::Unknown => Shape::Unknown,
        }
    }

    pub(crate) fn factory(&self, symbol: SymbolId) -> Option<&Expression<'a>> {
        if self.write(symbol).is_some() {
            return None;
        }
        let body = match self.nodes.kind(self.scoping.symbol_declaration(symbol)) {
            AstKind::VariableDeclarator(declaration) => {
                match unwrap_syntax_only(declaration.init.as_ref()?) {
                    Expression::ArrowFunctionExpression(arrow)
                        if arrow.params.items.is_empty()
                            && arrow.params.rest.is_none()
                            && !arrow.r#async =>
                    {
                        match &arrow.body {
                            ArrowFunctionBody::FunctionBody(body) => body,
                            expression => return expression.as_expression(),
                        }
                    }
                    Expression::FunctionExpression(function)
                        if function.params.items.is_empty()
                            && function.params.rest.is_none()
                            && !function.r#async
                            && !function.generator =>
                    {
                        function.body.as_ref()?
                    }
                    _ => return None,
                }
            }
            AstKind::Function(function)
                if function.params.items.is_empty()
                    && function.params.rest.is_none()
                    && !function.r#async
                    && !function.generator =>
            {
                function.body.as_ref()?
            }
            _ => return None,
        };
        match body.statements.as_slice() {
            [Statement::ReturnStatement(statement)] => statement.argument.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn write(&self, symbol: SymbolId) -> Option<u32> {
        self.scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .map(|reference| self.scoping.get_reference(*reference))
            .filter(|reference| reference.is_write())
            .map(|reference| self.nodes.kind(reference.node_id()).span().start)
            .min()
    }
}
