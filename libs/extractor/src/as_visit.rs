//! The element `<Component as={...} />` renders: a name the build knows, a
//! condition choosing between such names, or a type only the runtime gives

use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    Expression, JSXElement, JSXElementName, JSXIdentifier, JSXMemberExpressionObject, Str,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::LogicalOperator;

use crate::utils::unwrap_syntax_only;

/// What an element with an `as` becomes
pub enum As<'a> {
    /// The element renamed
    Name(JSXElementName<'a>),
    /// A condition choosing between the element renamed for each name
    Choice(Expression<'a>),
    /// A type only the runtime gives, or the default tag when it gives
    /// nothing (`undefined`, `null`, `false`)
    Runtime(Expression<'a>),
}

/// What an element whose `as` is `value` becomes; `default` is the tag it
/// renders without one
pub fn resolve<'a>(
    ast: &AstBuilder<'a>,
    element: &JSXElement<'a>,
    value: Expression<'a>,
    default: &str,
) -> As<'a> {
    if let Some(name) = element_name(ast, &value, default) {
        return As::Name(name);
    }
    if let Some(choice) = choice(ast, element, &value, default) {
        return As::Choice(choice);
    }
    As::Runtime(Expression::new_logical_expression(
        SPAN,
        Expression::new_parenthesized_expression(SPAN, value, ast),
        LogicalOperator::Or,
        Expression::new_string_literal(SPAN, Str::from_in(default, ast.allocator()), None, ast),
        ast,
    ))
}

/// Name `element` `name`
pub fn rename<'a>(ast: &AstBuilder<'a>, element: &mut JSXElement<'a>, name: JSXElementName<'a>) {
    if let Some(closing) = &mut element.closing_element {
        closing.name = name.clone_in(ast.allocator());
    }
    element.opening_element.name = name;
}

/// The name JSX gives `value`: a tag, a component, a member such as
/// `motion.div`, or the default tag for nothing
fn element_name<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    default: &str,
) -> Option<JSXElementName<'a>> {
    let tag = |name: &str| {
        JSXElementName::new_identifier(
            SPAN,
            Str::from_in(
                if name.is_empty() { default } else { name },
                ast.allocator(),
            ),
            ast,
        )
    };
    match unwrap_syntax_only(value) {
        Expression::StringLiteral(literal) => Some(tag(&literal.value)),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template.quasis
            [0]
        .value
        .cooked
        .as_ref()
        .map(|text| tag(text)),
        Expression::NullLiteral(_) => Some(tag("")),
        Expression::BooleanLiteral(literal) if !literal.value => Some(tag("")),
        Expression::Identifier(identifier) if identifier.name == "undefined" => Some(tag("")),
        // A name JSX reads as a binding rather than a tag
        Expression::Identifier(identifier)
            if !identifier
                .name
                .starts_with(|c: char| c.is_ascii_lowercase()) =>
        {
            Some(tag(&identifier.name))
        }
        Expression::StaticMemberExpression(member) => Some(JSXElementName::new_member_expression(
            SPAN,
            member_object(ast, &member.object)?,
            JSXIdentifier::new(
                SPAN,
                Str::from_in(member.property.name.as_str(), ast.allocator()),
                ast,
            ),
            ast,
        )),
        _ => None,
    }
}

fn member_object<'a>(
    ast: &AstBuilder<'a>,
    object: &Expression<'a>,
) -> Option<JSXMemberExpressionObject<'a>> {
    match unwrap_syntax_only(object) {
        Expression::Identifier(identifier) => Some(
            JSXMemberExpressionObject::new_identifier_reference(SPAN, identifier.name, ast),
        ),
        Expression::StaticMemberExpression(member) => {
            Some(JSXMemberExpressionObject::new_member_expression(
                SPAN,
                member_object(ast, &member.object)?,
                JSXIdentifier::new(
                    SPAN,
                    Str::from_in(member.property.name.as_str(), ast.allocator()),
                    ast,
                ),
                ast,
            ))
        }
        _ => None,
    }
}

/// `value` as `element` renamed for each name a condition chooses, when every
/// choice is a name
fn choice<'a>(
    ast: &AstBuilder<'a>,
    element: &JSXElement<'a>,
    value: &Expression<'a>,
    default: &str,
) -> Option<Expression<'a>> {
    if let Expression::ConditionalExpression(conditional) = unwrap_syntax_only(value) {
        return Some(Expression::new_conditional_expression(
            SPAN,
            conditional.test.clone_in(ast.allocator()),
            choice(ast, element, &conditional.consequent, default)?,
            choice(ast, element, &conditional.alternate, default)?,
            ast,
        ));
    }
    let mut renamed = element.clone_in(ast.allocator());
    rename(ast, &mut renamed, element_name(ast, value, default)?);
    Some(Expression::JSXElement(oxc_allocator::Box::new_in(
        renamed, ast,
    )))
}
