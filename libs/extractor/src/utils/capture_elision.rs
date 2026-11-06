//! Remove captures only when their sole read is eager and nothing observable
//! precedes it. Work backwards to preserve argument evaluation order.

use super::{rewrite_expression, unwrap_syntax_only, unwrap_syntax_only_mut};
use oxc_ast::ast::{
    ArrayExpressionElement, ChainElement, Expression, JSXAttributeItem, JSXAttributeValue,
    JSXChild, JSXElementName, ObjectPropertyKind,
};
use oxc_syntax::{identifier::is_identifier_name, operator::UnaryOperator};

type Captures<'a> = [(String, Expression<'a>)];

fn intrinsic(name: &JSXElementName<'_>) -> bool {
    matches!(name, JSXElementName::Identifier(name)
        if name.name.as_str().starts_with(|c: char| c.is_ascii_lowercase()))
}

/// Stringifying an already captured primitive cannot call user code again.
fn primitive<'b, 'a>(mut value: &'b Expression<'a>, locals: &'b Captures<'a>) -> bool {
    if let Expression::Identifier(identifier) = unwrap_syntax_only(value) {
        value = locals
            .iter()
            .find(|(name, _)| name == identifier.name.as_str())
            .map_or(value, |(_, original)| original);
    }
    loop {
        value = unwrap_syntax_only(value);
        if let Expression::SequenceExpression(sequence) = value
            && let Some(last) = sequence.expressions.last()
        {
            value = last;
        } else {
            break;
        }
    }
    matches!(
        value,
        Expression::TemplateLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flow {
    Inert,
    Read,
    Barrier,
}

impl Flow {
    /// This operation can observe its operands after they are read.
    fn observing(self) -> Self {
        if self == Self::Read {
            self
        } else {
            Self::Barrier
        }
    }
}

/// Whether the eager prefix reaches `name`, ends inertly, or encounters work.
fn prefix(value: &Expression<'_>, name: &str, locals: &Captures<'_>) -> Flow {
    match unwrap_syntax_only(value) {
        Expression::Identifier(identifier) => {
            if identifier.name == name {
                Flow::Read
            } else if locals
                .iter()
                .any(|(name, _)| name == identifier.name.as_str())
            {
                Flow::Inert
            } else {
                Flow::Barrier
            }
        }
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_) => Flow::Inert,
        Expression::LogicalExpression(logical) => {
            let left = prefix(&logical.left, name, locals);
            if left != Flow::Inert {
                left
            } else if prefix(&logical.right, name, locals) == Flow::Inert {
                Flow::Inert
            } else {
                Flow::Barrier
            }
        }
        Expression::ConditionalExpression(conditional) => {
            let test = prefix(&conditional.test, name, locals);
            if test != Flow::Inert {
                test
            } else if prefix(&conditional.consequent, name, locals) == Flow::Inert
                && prefix(&conditional.alternate, name, locals) == Flow::Inert
            {
                Flow::Inert
            } else {
                Flow::Barrier
            }
        }
        Expression::BinaryExpression(binary) => {
            let left = prefix(&binary.left, name, locals);
            if left == Flow::Inert {
                prefix(&binary.right, name, locals).observing()
            } else {
                left
            }
        }
        // `typeof` can suppress an unresolved read; `delete` observes references.
        Expression::UnaryExpression(unary)
            if !matches!(
                unary.operator,
                UnaryOperator::Typeof | UnaryOperator::Delete
            ) =>
        {
            prefix(&unary.argument, name, locals).observing()
        }
        Expression::StaticMemberExpression(member) => {
            prefix(&member.object, name, locals).observing()
        }
        Expression::ComputedMemberExpression(member) => {
            let object = prefix(&member.object, name, locals);
            if object != Flow::Inert {
                object
            } else if !member.optional {
                prefix(&member.expression, name, locals).observing()
            } else {
                Flow::Barrier
            }
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::StaticMemberExpression(member) => {
                prefix(&member.object, name, locals).observing()
            }
            ChainElement::ComputedMemberExpression(member) => {
                prefix(&member.object, name, locals).observing()
            }
            _ => Flow::Barrier,
        },
        Expression::TemplateLiteral(template) => {
            for value in &template.expressions {
                let flow = prefix(value, name, locals);
                if flow != Flow::Inert {
                    return flow;
                }
                if !primitive(value, locals) {
                    return Flow::Barrier;
                }
            }
            Flow::Inert
        }
        Expression::SequenceExpression(sequence) => {
            for value in &sequence.expressions {
                let flow = prefix(value, name, locals);
                if flow != Flow::Inert {
                    return flow;
                }
            }
            Flow::Inert
        }
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                match element {
                    ArrayExpressionElement::Elision(_) => {}
                    ArrayExpressionElement::SpreadElement(spread) => {
                        return prefix(&spread.argument, name, locals).observing();
                    }
                    element => {
                        if let Some(value) = element.as_expression() {
                            let flow = prefix(value, name, locals);
                            if flow != Flow::Inert {
                                return flow;
                            }
                        }
                    }
                }
            }
            Flow::Inert
        }
        Expression::ObjectExpression(object) => {
            for property in &object.properties {
                let value = match property {
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        return prefix(&spread.argument, name, locals).observing();
                    }
                    ObjectPropertyKind::ObjectProperty(property) if !property.computed => {
                        &property.value
                    }
                    ObjectPropertyKind::ObjectProperty(_) => return Flow::Barrier,
                };
                let flow = prefix(value, name, locals);
                if flow != Flow::Inert {
                    return flow;
                }
            }
            Flow::Inert
        }
        Expression::JSXElement(element) if intrinsic(&element.opening_element.name) => {
            for attribute in &element.opening_element.attributes {
                let value = match attribute {
                    JSXAttributeItem::SpreadAttribute(spread) => {
                        return prefix(&spread.argument, name, locals).observing();
                    }
                    JSXAttributeItem::Attribute(attribute) => match &attribute.value {
                        Some(JSXAttributeValue::ExpressionContainer(container)) => {
                            container.expression.as_expression()
                        }
                        Some(JSXAttributeValue::Element(_) | JSXAttributeValue::Fragment(_)) => {
                            return Flow::Barrier;
                        }
                        _ => None,
                    },
                };
                if let Some(value) = value {
                    let flow = prefix(value, name, locals);
                    if flow != Flow::Inert {
                        return flow;
                    }
                }
            }
            for child in &element.children {
                match child {
                    JSXChild::ExpressionContainer(container) => {
                        if let Some(value) = container.expression.as_expression() {
                            let flow = prefix(value, name, locals);
                            if flow != Flow::Inert {
                                return flow;
                            }
                        }
                    }
                    JSXChild::Text(_) => {}
                    _ => return Flow::Barrier,
                }
            }
            Flow::Barrier
        }
        _ => Flow::Barrier,
    }
}

/// Count expression reads while rejecting scopes and opaque generated code.
fn reads(expression: &mut Expression<'_>, name: &str) -> Option<usize> {
    let mut count = 0;
    let mut blocked = false;
    rewrite_expression(expression, &mut |value| {
        blocked |= match value {
            Expression::Identifier(identifier) => {
                count += usize::from(identifier.name == name);
                // Some style slots encode complete JS expressions in the name.
                !is_identifier_name(identifier.name.as_str())
            }
            Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::AssignmentExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::AwaitExpression(_)
            | Expression::YieldExpression(_) => true,
            Expression::JSXElement(element) => !intrinsic(&element.opening_element.name),
            Expression::CallExpression(call) => matches!(unwrap_syntax_only(&call.callee),
                Expression::Identifier(identifier) if identifier.name == "eval"),
            _ => false,
        };
        false
    });
    (!blocked).then_some(count)
}

pub(super) fn inline<'a>(values: &mut Vec<(String, Expression<'a>)>, body: &mut Expression<'a>) {
    while let Some((name, mut value)) = values.pop() {
        if reads(&mut value, "").is_none()
            || reads(body, &name) != Some(1)
            || prefix(body, &name, values) != Flow::Read
        {
            values.push((name, value));
            break;
        }
        let mut replacement = Some(value);
        rewrite_expression(body, &mut |expression| {
            if let Expression::TemplateLiteral(template) = expression {
                for slot in &mut template.expressions {
                    if matches!(unwrap_syntax_only(slot), Expression::Identifier(identifier) if identifier.name == name.as_str())
                        && let Some(Expression::TemplateLiteral(value)) =
                            replacement.as_mut().map(unwrap_syntax_only_mut)
                        && value.quasis.len() == 2
                        && value.quasis.iter().all(|quasi| {
                            quasi
                                .value
                                .cooked
                                .is_some_and(|text| text.as_str() == Some(""))
                        })
                        && let Some(value) = value.expressions.pop()
                    {
                        // The enclosing untagged interpolation already performs ToString.
                        *slot = value;
                        replacement = None;
                    }
                }
            }
            if matches!(expression, Expression::Identifier(identifier) if identifier.name == name.as_str())
                && let Some(value) = replacement.take()
            {
                *expression = value;
                return true;
            }
            if let Expression::ObjectExpression(object) = expression {
                for property in &mut object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && matches!(&property.value, Expression::Identifier(identifier) if identifier.name == name.as_str())
                    {
                        property.shorthand = false;
                    }
                }
            }
            false
        });
    }
}
