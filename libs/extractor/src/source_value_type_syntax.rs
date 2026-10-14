use std::collections::BTreeMap;

use oxc_ast::ast::{
    Expression, Function, IdentifierReference, ObjectPropertyKind, PropertyKind, TemplateElement,
};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;

use super::{
    annotation,
    model::{Node, Scalar},
};

pub(super) fn reference(identifier: &IdentifierReference<'_>, scoping: &Scoping) -> Node {
    identifier
        .reference_id
        .get()
        .and_then(|id| scoping.get_reference(id).symbol_id())
        .map_or(Node::UNKNOWN, Node::Symbol)
}

pub(super) fn expression(value: &Expression<'_>, scoping: &Scoping) -> Node {
    let at = |value: &Expression<'_>| Node::Expression(value.span());
    match value {
        Expression::NumericLiteral(_) => Node::Scalar(Scalar::Number),
        Expression::StringLiteral(value) => Node::Scalar(string(&value.value)),
        Expression::TemplateLiteral(value) => {
            Node::Scalar(template(&value.quasis, !value.expressions.is_empty()))
        }
        Expression::BigIntLiteral(_) => Node::Scalar(Scalar::BigInt),
        Expression::Identifier(identifier) => reference(identifier, scoping),
        Expression::ParenthesizedExpression(inner) => at(&inner.expression),
        Expression::TSAsExpression(inner) => at(&inner.expression),
        Expression::TSSatisfiesExpression(inner) => at(&inner.expression),
        Expression::TSNonNullExpression(inner) => at(&inner.expression),
        Expression::TSTypeAssertion(inner) => at(&inner.expression),
        Expression::TSInstantiationExpression(inner) => at(&inner.expression),
        Expression::BinaryExpression(binary) => Node::Binary(
            binary.operator,
            Box::new(at(&binary.left)),
            Box::new(at(&binary.right)),
        ),
        Expression::UnaryExpression(unary) => {
            Node::Unary(unary.operator, Box::new(at(&unary.argument)))
        }
        Expression::ConditionalExpression(branch) => {
            Node::Union(vec![at(&branch.consequent), at(&branch.alternate)])
        }
        Expression::LogicalExpression(logical) => {
            Node::Union(vec![at(&logical.left), at(&logical.right)])
        }
        Expression::SequenceExpression(sequence) => {
            sequence.expressions.last().map_or(Node::UNKNOWN, at)
        }
        Expression::StaticMemberExpression(member) if !member.optional => {
            at(&member.object).member(member.property.name.to_string())
        }
        Expression::ComputedMemberExpression(member) if !member.optional => {
            key(&member.expression).map_or(Node::UNKNOWN, |key| at(&member.object).member(key))
        }
        Expression::CallExpression(call) if !call.optional => {
            Node::Call(Box::new(at(&call.callee)))
        }
        Expression::FunctionExpression(function) => function_type(function, scoping),
        Expression::ArrowFunctionExpression(function)
            if !function.r#async && function.type_parameters.is_none() =>
        {
            Node::Function(Box::new(
                function
                    .return_type
                    .as_ref()
                    .map_or(Node::UNKNOWN, |annotation| {
                        annotation::ty(&annotation.type_annotation, scoping)
                    }),
            ))
        }
        Expression::ObjectExpression(object) => {
            let mut fields = BTreeMap::new();
            for property in &object.properties {
                match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        let Some(key) = property.key.static_name() else {
                            return Node::UNKNOWN;
                        };
                        let value = match property.kind {
                            PropertyKind::Init => at(&property.value),
                            PropertyKind::Get => Node::Call(Box::new(at(&property.value))),
                            PropertyKind::Set => Node::UNKNOWN,
                        };
                        fields.insert(key.to_string(), value);
                    }
                    ObjectPropertyKind::SpreadProperty(_) => return Node::UNKNOWN,
                }
            }
            Node::Object(fields)
        }
        Expression::ArrayExpression(array) => {
            let mut fields = BTreeMap::new();
            for (index, item) in array.elements.iter().enumerate() {
                if item.is_spread() {
                    return Node::UNKNOWN;
                }
                fields.insert(
                    index.to_string(),
                    item.as_expression().map_or(Node::UNKNOWN, at),
                );
            }
            Node::Object(fields)
        }
        _ => Node::UNKNOWN,
    }
}

pub(super) fn key(value: &Expression<'_>) -> Option<String> {
    match value {
        Expression::StringLiteral(value) => Some(value.value.to_string()),
        Expression::NumericLiteral(value) => Some(crate::utils::js_number_string(value.value)),
        _ => None,
    }
}

pub(super) fn function_type(function: &Function<'_>, scoping: &Scoping) -> Node {
    if function.r#async || function.generator || function.type_parameters.is_some() {
        return Node::UNKNOWN;
    }
    Node::Function(Box::new(
        function
            .return_type
            .as_ref()
            .map_or(Node::UNKNOWN, |annotation| {
                annotation::ty(&annotation.type_annotation, scoping)
            }),
    ))
}

pub(super) fn string(value: &str) -> Scalar {
    if css::numeric_value::parse(value).is_some() {
        Scalar::String
    } else {
        Scalar::NonNumericString
    }
}

pub(super) fn template(quasis: &[TemplateElement<'_>], interpolated: bool) -> Scalar {
    let Some(last) = quasis.last().and_then(|quasi| quasi.value.cooked.as_ref()) else {
        return Scalar::String;
    };
    if !interpolated && quasis.len() == 1 {
        return string(last);
    }
    if [
        "px", "em", "rem", "vh", "vw", "vmin", "vmax", "%", "ms", "s", "ch", "ex", "cm", "mm",
        "in", "pt", "pc", "dvh", "dvw", "svh", "svw", "lvh", "lvw",
    ]
    .iter()
    .any(|unit| {
        last.len()
            .checked_sub(unit.len())
            .and_then(|start| last.get(start..))
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case(unit))
    }) {
        Scalar::NonNumericString
    } else {
        Scalar::String
    }
}
