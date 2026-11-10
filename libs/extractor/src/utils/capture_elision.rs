//! Inline eager single-use captures and preserve unused arguments as sequences.
//! Work backwards to preserve evaluation order and suspension scope.

use super::{rewrite_expression, unwrap_syntax_only, unwrap_syntax_only_mut};
use oxc_allocator::{FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    ArrayExpressionElement, AssignmentTarget, ChainElement, Expression, JSXAttributeItem,
    JSXAttributeValue, JSXChild, JSXElementName, ObjectPropertyKind, SimpleAssignmentTarget,
};
use oxc_ast::builder::AstBuilder;
use oxc_parser::Parser;
use oxc_span::{SPAN, SourceType};
use oxc_syntax::{
    identifier::is_identifier_name,
    operator::{BinaryOperator, UnaryOperator},
};

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

/// These literals cannot invoke coercion hooks or mix `BigInt` with Number.
fn scalar_literal(value: &Expression<'_>) -> bool {
    super::js_number_literal(value).is_some()
        || matches!(
            unwrap_syntax_only(value),
            Expression::StringLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
        )
        || matches!(unwrap_syntax_only(value), Expression::TemplateLiteral(template) if template.expressions.is_empty())
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
        | Expression::BigIntLiteral(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_) => Flow::Inert,
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
                let right = prefix(&binary.right, name, locals);
                if right == Flow::Inert
                    && !matches!(
                        binary.operator,
                        BinaryOperator::In | BinaryOperator::Instanceof
                    )
                    && scalar_literal(&binary.left)
                    && scalar_literal(&binary.right)
                {
                    Flow::Inert
                } else {
                    right.observing()
                }
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
                    ObjectPropertyKind::ObjectProperty(property) => {
                        if property.computed {
                            let flow = property.key.as_expression().map_or(Flow::Barrier, |key| {
                                let flow = prefix(key, name, locals);
                                if flow == Flow::Inert && !primitive(key, locals) {
                                    Flow::Barrier
                                } else {
                                    flow
                                }
                            });
                            if flow != Flow::Inert {
                                return flow;
                            }
                        }
                        &property.value
                    }
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

/// Suspension may be inlined only when removing the final generated scope.
#[derive(Clone, Copy)]
enum Scan {
    Body,
    Transport,
    Argument { can_suspend: bool },
}

fn reads(expression: &mut Expression<'_>, name: &str, scan: Scan) -> Option<usize> {
    let mut count = 0;
    let mut blocked = false;
    rewrite_expression(expression, &mut |value| {
        if matches!(scan, Scan::Transport)
            && matches!(
                value,
                Expression::ArrowFunctionExpression(_)
                    | Expression::FunctionExpression(_)
                    | Expression::ClassExpression(_)
            )
        {
            blocked |= reads(value, name, Scan::Body) != Some(0);
            return true;
        }
        blocked |= match value {
            Expression::Identifier(identifier) => {
                count += usize::from(identifier.name == name);
                !is_identifier_name(identifier.name.as_str())
            }
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                matches!(scan, Scan::Argument { .. })
            }
            Expression::ClassExpression(_) => matches!(scan, Scan::Argument { .. }),
            Expression::AwaitExpression(_) | Expression::YieldExpression(_) => {
                matches!(scan, Scan::Argument { can_suspend: false })
            }
            // Plain assignment targets are not expression reads. Keep parameters
            // that are written, including targets nested in destructuring patterns.
            Expression::AssignmentExpression(assignment)
                if !matches!(scan, Scan::Argument { .. }) =>
            {
                match &assignment.left {
                    AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                        identifier.name == name
                    }
                    pattern @ (AssignmentTarget::ArrayAssignmentTarget(_)
                    | AssignmentTarget::ObjectAssignmentTarget(_)) => writes(pattern, name),
                    _ => false,
                }
            }
            Expression::UpdateExpression(update) if !matches!(scan, Scan::Argument { .. }) => {
                matches!(&update.argument, SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) if identifier.name == name)
            }
            Expression::JSXElement(element) => {
                if matches!(scan, Scan::Argument { .. }) {
                    !intrinsic(&element.opening_element.name)
                } else {
                    match &element.opening_element.name {
                        JSXElementName::Identifier(identifier) => {
                            count += usize::from(identifier.name == name);
                            false
                        }
                        JSXElementName::IdentifierReference(identifier) => {
                            count += usize::from(identifier.name == name);
                            false
                        }
                        _ => matches!(scan, Scan::Body),
                    }
                }
            }
            Expression::CallExpression(call) => matches!(unwrap_syntax_only(&call.callee),
                Expression::Identifier(identifier) if identifier.name == "eval"),
            _ => false,
        };
        false
    });
    (!blocked).then_some(count)
}

fn writes(target: &AssignmentTarget<'_>, name: &str) -> bool {
    let element = |value: &oxc_ast::ast::AssignmentTargetMaybeDefault<'_>| {
        value
            .as_assignment_target()
            .is_none_or(|target| writes(target, name))
    };
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => id.name == name,
        AssignmentTarget::ArrayAssignmentTarget(array) => {
            array.elements.iter().flatten().any(element)
                || array
                    .rest
                    .as_ref()
                    .is_some_and(|rest| writes(&rest.target, name))
        }
        AssignmentTarget::ObjectAssignmentTarget(object) => {
            object.properties.iter().any(|property| match property {
                oxc_ast::ast::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
                    property,
                ) => property.binding.name == name || property.init.is_some(),
                oxc_ast::ast::AssignmentTargetProperty::AssignmentTargetPropertyProperty(
                    property,
                ) => property.computed || element(&property.binding),
            }) || object
                .rest
                .as_ref()
                .is_some_and(|rest| writes(&rest.target, name))
        }
        _ => false,
    }
}

/// Moving eager reads into their existing statement scope must not turn a
/// per-evaluation closure binding into one shared by later loop iterations.
pub(crate) fn can_transport(body: &mut Expression<'_>, values: &Captures<'_>) -> bool {
    values
        .iter()
        .all(|(name, _)| can_transport_name(body, name))
}

pub(crate) fn can_transport_name(body: &mut Expression<'_>, name: &str) -> bool {
    reads(body, name, Scan::Transport).is_some()
}

pub(crate) fn inline<'a>(
    builder: &AstBuilder<'a>,
    values: &mut Vec<(String, Expression<'a>)>,
    body: &mut Expression<'a>,
) {
    if values.is_empty() {
        return;
    }
    // Generated style slots can carry a whole expression in an identifier's text.
    // Decode them before counting or replacing references inside that expression.
    rewrite_expression(body, &mut |expression| {
        if let Expression::Identifier(identifier) = expression
            && !is_identifier_name(identifier.name.as_str())
            && let Ok(mut parsed) = Parser::new(
                builder.allocator(),
                identifier.name.as_str(),
                SourceType::tsx(),
            )
            .parse_expression()
            && reads(&mut parsed, "", Scan::Argument { can_suspend: false }).is_some()
        {
            super::clear_generated_spans(&mut parsed);
            *expression = parsed;
            return true;
        }
        false
    });
    for index in 0..values.len() {
        let name = values[index].0.clone();
        if !name.starts_with("__devupChoice")
            || reads(body, &name, Scan::Body) != Some(1)
            || values.iter_mut().enumerate().any(|(other, (_, value))| {
                other != index && reads(value, &name, Scan::Body) != Some(0)
            })
        {
            continue;
        }
        rewrite_expression(body, &mut |expression| {
            if let Expression::ComputedMemberExpression(member) = expression
                && !member.optional
                && let Some(slot) = super::selection_projection::slot(&member.expression)
                && matches!(unwrap_syntax_only(&slot.object), Expression::Identifier(id) if id.name == name.as_str())
                && let Some(projected) =
                    super::selection_projection::project(builder, &member.object, &values[index].1)
            {
                values[index].1 = projected;
                *expression = Expression::new_identifier(
                    SPAN,
                    oxc_ast::ast::Ident::from_in(name.as_str(), builder.allocator()),
                    builder,
                );
                return true;
            }
            false
        });
    }
    while let Some((name, mut value)) = values.pop() {
        let scan = Scan::Argument {
            can_suspend: values.is_empty(),
        };
        let count = reads(body, &name, Scan::Body);
        if count == Some(0) && reads(&mut value, "", scan).is_some() {
            // The argument still runs before the body even when its result is unused.
            *body = Expression::new_sequence_expression(
                SPAN,
                oxc_allocator::Vec::from_array_in([value, body.take_in(builder)], builder),
                builder,
            );
            continue;
        }
        if reads(&mut value, "", scan).is_none()
            || count != Some(1)
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
    super::selection_projection::direct(builder, body);
}
