use oxc_ast::ast::{Argument, Expression, ModuleExportName, PropertyKey};

use super::{scopes::Scopes, text::Text, wire::Origin};

pub(super) fn origin(node: &Expression<'_>, scopes: &Scopes, scope: usize) -> Option<Origin> {
    match node {
        Expression::Identifier(id) => scopes.binding(scope, id.name.as_str()),
        Expression::CallExpression(call) => {
            let Expression::Identifier(callee) = &call.callee else {
                return None;
            };
            if callee.name != "require"
                || scopes.owner(scope, "require").is_some()
                || call.arguments.len() != 1
            {
                return None;
            }
            let Argument::StringLiteral(request) = &call.arguments[0] else {
                return None;
            };
            if request.value.is_empty() {
                return None;
            }
            Some(Origin {
                request: Text::literal(request),
                ids: Vec::new(),
                require: true,
            })
        }
        Expression::StaticMemberExpression(member) => {
            let mut value = origin(&member.object, scopes, scope)?;
            value.ids.push(Text::from(member.property.name.as_str()));
            Some(value)
        }
        Expression::ComputedMemberExpression(member) => {
            let name = literal_name(&member.expression)?;
            let mut value = origin(&member.object, scopes, scope)?;
            value.ids.push(name);
            Some(value)
        }
        Expression::PrivateFieldExpression(member) => {
            let mut value = origin(&member.object, scopes, scope)?;
            value.ids.push(Text::from(member.field.name.as_str()));
            Some(value)
        }
        _ => None,
    }
}

pub(super) fn literal_name(node: &Expression<'_>) -> Option<Text> {
    match node {
        Expression::StringLiteral(value) => Some(Text::literal(value)),
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_) => Some(Text::from("")),
        _ => None,
    }
}

pub(super) fn property_name(key: &PropertyKey<'_>, computed: bool) -> Option<Text> {
    match key {
        PropertyKey::StaticIdentifier(id) if !computed => Some(Text::from(id.name.as_str())),
        PropertyKey::PrivateIdentifier(id) if !computed => Some(Text::from(id.name.as_str())),
        _ => key.as_expression().and_then(literal_name),
    }
}

pub(super) fn export_name(name: &ModuleExportName<'_>) -> Text {
    match name {
        ModuleExportName::IdentifierName(id) => Text::from(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Text::from(id.name.as_str()),
        ModuleExportName::StringLiteral(value) => Text::literal(value),
    }
}

pub(super) fn local_name<'n>(name: &'n ModuleExportName<'_>) -> Option<&'n str> {
    match name {
        ModuleExportName::IdentifierName(id) => Some(id.name.as_str()),
        ModuleExportName::IdentifierReference(id) => Some(id.name.as_str()),
        ModuleExportName::StringLiteral(_) => None,
    }
}
