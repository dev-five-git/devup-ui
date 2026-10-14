use std::collections::BTreeMap;

use oxc_ast::ast::{TSLiteral, TSSignature, TSType, TSTypeName};
use oxc_semantic::Scoping;

use super::{
    model::{Node, Scalar},
    syntax,
};

pub(super) fn name(value: &TSTypeName<'_>, scoping: &Scoping) -> Node {
    match value {
        TSTypeName::IdentifierReference(identifier) => syntax::reference(identifier, scoping),
        TSTypeName::QualifiedName(qualified) => {
            name(&qualified.left, scoping).member(qualified.right.name.to_string())
        }
        TSTypeName::ThisExpression(_) => Node::UNKNOWN,
    }
}

pub(super) fn ty(value: &TSType<'_>, scoping: &Scoping) -> Node {
    match value {
        TSType::TSNumberKeyword(_) => Node::Scalar(Scalar::Number),
        TSType::TSStringKeyword(_) => Node::Scalar(Scalar::String),
        TSType::TSTemplateLiteralType(value) => {
            Node::Scalar(syntax::template(&value.quasis, !value.types.is_empty()))
        }
        TSType::TSBigIntKeyword(_) => Node::Scalar(Scalar::BigInt),
        TSType::TSLiteralType(literal) => match &literal.literal {
            TSLiteral::NumericLiteral(_) => Node::Scalar(Scalar::Number),
            TSLiteral::BigIntLiteral(_) => Node::Scalar(Scalar::BigInt),
            TSLiteral::StringLiteral(value) => Node::Scalar(syntax::string(&value.value)),
            TSLiteral::TemplateLiteral(value) => Node::Scalar(syntax::template(
                &value.quasis,
                !value.expressions.is_empty(),
            )),
            TSLiteral::UnaryExpression(unary) => syntax::expression(&unary.argument, scoping),
            TSLiteral::BooleanLiteral(_) => Node::UNKNOWN,
        },
        TSType::TSParenthesizedType(inner) => ty(&inner.type_annotation, scoping),
        TSType::TSUnionType(union) => {
            Node::Union(union.types.iter().map(|item| ty(item, scoping)).collect())
        }
        TSType::TSIntersectionType(intersection) => Node::Merge(
            intersection
                .types
                .iter()
                .map(|item| ty(item, scoping))
                .collect(),
        ),
        TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
            name(&reference.type_name, scoping)
        }
        TSType::TSTypeLiteral(object) => members(&object.members, scoping),
        TSType::TSFunctionType(function) if function.type_parameters.is_none() => {
            Node::Function(Box::new(ty(&function.return_type.type_annotation, scoping)))
        }
        TSType::TSTypeOperatorType(operator)
            if operator.operator == oxc_ast::ast::TSTypeOperatorOperator::Readonly =>
        {
            ty(&operator.type_annotation, scoping)
        }
        _ => Node::UNKNOWN,
    }
}

pub(super) fn members(values: &[TSSignature<'_>], scoping: &Scoping) -> Node {
    let mut fields = BTreeMap::new();
    for value in values {
        let (key, node) = match value {
            TSSignature::TSPropertySignature(property) => (
                property.key.static_name(),
                if property.optional {
                    Node::UNKNOWN
                } else {
                    property
                        .type_annotation
                        .as_ref()
                        .map_or(Node::UNKNOWN, |annotation| {
                            ty(&annotation.type_annotation, scoping)
                        })
                },
            ),
            TSSignature::TSMethodSignature(method) => (
                method.key.static_name(),
                if method.optional || method.type_parameters.is_some() {
                    Node::UNKNOWN
                } else {
                    let result = method
                        .return_type
                        .as_ref()
                        .map_or(Node::UNKNOWN, |annotation| {
                            ty(&annotation.type_annotation, scoping)
                        });
                    match method.kind {
                        oxc_ast::ast::TSMethodSignatureKind::Method => {
                            Node::Function(Box::new(result))
                        }
                        oxc_ast::ast::TSMethodSignatureKind::Get => result,
                        oxc_ast::ast::TSMethodSignatureKind::Set => Node::UNKNOWN,
                    }
                },
            ),
            TSSignature::TSIndexSignature(_)
            | TSSignature::TSCallSignatureDeclaration(_)
            | TSSignature::TSConstructSignatureDeclaration(_) => continue,
        };
        if let Some(key) = key {
            fields.insert(key.to_string(), node);
        }
    }
    Node::Object(fields)
}
