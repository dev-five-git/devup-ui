//! Emotion's `target` and styled-components' `componentId`: the stable
//! classes a component renders, read the way each library reads them

use super::{marker_class, member, option_properties, with_property};
use crate::utils::{
    build_time_error, get_string_by_literal_expression, readable_code, unwrap_syntax_only,
    wrap_direct_call,
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{
        ArrayExpressionElement, BindingPattern, Expression, FormalParameter, FormalParameterKind,
        FormalParameterRest, FormalParameters, IdentifierName, IdentifierReference,
        ObjectPropertyKind, StaticMemberExpression, TSTypeAnnotation, TSTypeParameterDeclaration,
    },
    builder::AstBuilder,
};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::LogicalOperator;

/// What the identity options of a component give it
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Identity {
    /// The classes `componentId` gives it, as the `class` attribute splits
    /// them, which a component extending it renders too
    pub classes: Vec<String>,
    /// The classes Emotion's `target` gives it alone: a component extending
    /// it renders its base without them
    pub targets: Vec<String>,
    /// The `styledComponentId` styled-components gives it, when `componentId`
    /// names it
    pub component_id: Option<String>,
}

/// The property a component with Emotion targets holds itself without them
/// in, which a component extending it renders, as Emotion renders the base
/// an `__emotion_real` component holds
pub const BASE: &str = "__devupBase";

/// Whether an identifier reads the global of its name, not a binding
pub type Global<'g> = &'g dyn Fn(&IdentifierReference<'_>) -> bool;

/// A literal option value, as JavaScript reads it
#[derive(Debug, PartialEq, Eq)]
enum Literal {
    Undefined,
    String(String),
    /// Any other value: what `String(value)` writes, and whether it is truthy
    Other(String, bool),
}

const DYNAMIC: &str = "`target`, `componentId` and a `displayName` beside a `componentId` decide the classes the component renders; write them as literals";

const NOT_STRING_ID: &str =
    "styled-components throws on a `componentId` that is not a string; write it as a string";

const NOT_STRING_NAME: &str = "styled-components throws on a truthy `displayName` that is not a string beside a `componentId`; write it as a string";

/// The identity the options objects give a component, each option the last
/// one written, with the errors of those the build cannot read
pub fn read_identity(
    options: &[&Expression<'_>],
    global: Global<'_>,
) -> (Identity, Vec<(u32, String)>) {
    let mut identity = Identity::default();
    let mut errors = Vec::new();
    if let Some(target) = last_option(options, &["target"]) {
        match literal(target, global) {
            Some(value) => identity.targets.extend(classes(&target_text(value))),
            None => errors.push(error(target, DYNAMIC)),
        }
    }
    if let Some(component_id) = last_option(options, &["componentId"]) {
        let display_name = last_option(options, &["displayName"]);
        match styled_component_id(component_id, display_name, global) {
            Ok(Some(id)) => {
                identity.classes.extend(classes(&id));
                identity.component_id = Some(id);
            }
            Ok(None) => {}
            Err(error) => errors.push(error),
        }
    }
    (identity, errors)
}

/// `component` with the statics its identity gives it:
/// `Object.assign(component, { styledComponentId: id, __devupBase: base })`
pub fn with_statics<'a>(
    ast_builder: &AstBuilder<'a>,
    component: Expression<'a>,
    id: Option<&str>,
    base: Option<Expression<'a>>,
) -> Expression<'a> {
    if id.is_none() && base.is_none() {
        return component;
    }
    let mut statics = Expression::new_object_expression(
        SPAN,
        oxc_allocator::Vec::new_in(ast_builder),
        ast_builder,
    );
    if let Some(id) = id {
        statics = with_property(
            ast_builder,
            statics,
            "styledComponentId",
            marker_class(ast_builder, id),
        );
    }
    if let Some(base) = base {
        statics = with_property(ast_builder, statics, BASE, base);
    }
    wrap_direct_call(
        ast_builder,
        &member(ast_builder, "Object", "assign"),
        &[component, statics],
    )
}

/// `component.__devupBase`: a component with Emotion targets, without them
pub fn base_of<'a>(ast_builder: &AstBuilder<'a>, component: Expression<'a>) -> Expression<'a> {
    Expression::StaticMemberExpression(StaticMemberExpression::boxed(
        SPAN,
        component,
        IdentifierName::new(SPAN, BASE, ast_builder),
        false,
        ast_builder,
    ))
}

/// What a component extending `component` renders when only the runtime
/// knows whether Emotion's `target` gives it classes, as Emotion checks
/// `__emotion_real` at runtime: `component.__devupBase || component`, read
/// once through `((c) => c.__devupBase || c)(component)` unless it is a binding
pub fn runtime_base<'a>(
    ast_builder: &AstBuilder<'a>,
    component: &Expression<'a>,
) -> Expression<'a> {
    match component {
        Expression::Identifier(_) => base_or_self(ast_builder, component),
        component => read_once(ast_builder, component),
    }
}

fn base_or_self<'a>(ast_builder: &AstBuilder<'a>, component: &Expression<'a>) -> Expression<'a> {
    let allocator = ast_builder.allocator();
    Expression::new_logical_expression(
        SPAN,
        base_of(ast_builder, component.clone_in(allocator)),
        LogicalOperator::Or,
        component.clone_in(allocator),
        ast_builder,
    )
}

fn read_once<'a>(ast_builder: &AstBuilder<'a>, component: &Expression<'a>) -> Expression<'a> {
    let parameter = FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(ast_builder),
        BindingPattern::new_binding_identifier(SPAN, "c", ast_builder),
        None::<oxc_allocator::Box<TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        ast_builder,
    );
    let params = FormalParameters::boxed(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_array_in([parameter], ast_builder),
        None::<oxc_allocator::Box<FormalParameterRest<'a>>>,
        ast_builder,
    );
    let body = base_or_self(
        ast_builder,
        &Expression::new_identifier(SPAN, "c", ast_builder),
    );
    let read = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<TSTypeAnnotation<'a>>>,
        body.into(),
        ast_builder,
    );
    wrap_direct_call(
        ast_builder,
        &read,
        &[component.clone_in(ast_builder.allocator())],
    )
}

fn last_option<'b, 'a>(
    options: &'b [&'b Expression<'a>],
    name: &'static [&'static str],
) -> Option<&'b Expression<'a>> {
    option_properties(options, name)
        .last()
        .map(|property| &property.value)
}

fn error(value: &Expression<'_>, requirement: &str) -> (u32, String) {
    (
        value.span().start,
        build_time_error("styled", &readable_code(value), requirement),
    )
}

/// The classes a class string gives, split at ASCII whitespace as the `class`
/// attribute splits it
fn classes(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_ascii_whitespace().map(str::to_string)
}

/// The text Emotion appends to the classes for `target`: `String(target)`,
/// nothing when it is `undefined`
fn target_text(value: Literal) -> String {
    match value {
        Literal::Undefined => String::new(),
        Literal::String(text) | Literal::Other(text, _) => text,
    }
}

/// The `styledComponentId` styled-components gives a component for its
/// `componentId` and `displayName`:
/// `displayName && componentId ? escape(displayName) + '-' + componentId : componentId`
fn styled_component_id(
    component_id: &Expression<'_>,
    display_name: Option<&Expression<'_>>,
    global: Global<'_>,
) -> Result<Option<String>, (u32, String)> {
    let id = match literal(component_id, global) {
        Some(Literal::Undefined) => return Ok(None),
        Some(Literal::String(id)) => id,
        Some(Literal::Other(..)) => return Err(error(component_id, NOT_STRING_ID)),
        None => return Err(error(component_id, DYNAMIC)),
    };
    if id.is_empty() {
        return Ok(Some(id));
    }
    let Some(display_name) = display_name else {
        return Ok(Some(id));
    };
    match literal(display_name, global) {
        Some(Literal::String(name)) if !name.is_empty() => {
            Ok(Some(format!("{}-{id}", escape(&name))))
        }
        Some(Literal::Undefined | Literal::String(_) | Literal::Other(_, false)) => Ok(Some(id)),
        Some(Literal::Other(_, true)) => Err(error(display_name, NOT_STRING_NAME)),
        None => Err(error(display_name, DYNAMIC)),
    }
}

/// styled-components' `escape`: each run of the punctuation a CSS identifier
/// cannot hold becomes one `-`, and a `-` at either end is dropped
fn escape(name: &str) -> String {
    let mut escaped = String::with_capacity(name.len());
    for char in name.chars() {
        if !"!\"#$%&'()*+,./:;<=>?@[\\]^`{|}~-".contains(char) {
            escaped.push(char);
        } else if !escaped.ends_with('-') {
            escaped.push('-');
        }
    }
    let escaped = escaped.strip_prefix('-').unwrap_or(&escaped);
    escaped.strip_suffix('-').unwrap_or(escaped).to_string()
}

/// `value` as JavaScript reads it, when it is a literal
fn literal(value: &Expression<'_>, global: Global<'_>) -> Option<Literal> {
    let value = unwrap_syntax_only(value);
    match value {
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
            get_string_by_literal_expression(value).map(|text| Literal::String(text.into_owned()))
        }
        Expression::Identifier(identifier) if global(identifier) => global_literal(identifier),
        Expression::NullLiteral(_) => Some(Literal::Other("null".to_string(), false)),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .map(|element| element_text(element, global))
            .collect::<Option<Vec<_>>>()
            .map(|parts| Literal::Other(parts.join(","), true)),
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .all(plain_property)
            .then(|| Literal::Other("[object Object]".to_string(), true)),
        _ => get_string_by_literal_expression(value).map(|text| {
            let truthy = !matches!(text.as_ref(), "false" | "0" | "NaN");
            Literal::Other(text.into_owned(), truthy)
        }),
    }
}

/// The globals naming a value
fn global_literal(identifier: &IdentifierReference<'_>) -> Option<Literal> {
    match identifier.name.as_str() {
        "undefined" => Some(Literal::Undefined),
        "NaN" => Some(Literal::Other("NaN".to_string(), false)),
        "Infinity" => Some(Literal::Other("Infinity".to_string(), true)),
        _ => None,
    }
}

/// What `Array.prototype.join` writes for an element: nothing for a hole,
/// `null` or `undefined`
fn element_text(element: &ArrayExpressionElement<'_>, global: Global<'_>) -> Option<String> {
    match element {
        ArrayExpressionElement::Elision(_) => Some(String::new()),
        ArrayExpressionElement::SpreadElement(_) => None,
        element => match unwrap_syntax_only(element.to_expression()) {
            Expression::NullLiteral(_) => Some(String::new()),
            value => literal(value, global).map(target_text),
        },
    }
}

/// Whether a property of an object literal leaves it writing
/// `[object Object]`: it does not set its prototype or how it converts
fn plain_property(property: &ObjectPropertyKind<'_>) -> bool {
    match property {
        ObjectPropertyKind::ObjectProperty(property) => {
            !property.computed
                && !["toString", "valueOf", "__proto__"]
                    .iter()
                    .any(|name| property.key.is_specific_static_name(name))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("Card", "Card")]
    #[case("My Card!", "My Card")]
    #[case("a..b", "a-b")]
    #[case("-a-", "a")]
    #[case("--a", "a")]
    #[case("-", "")]
    #[case("!!", "")]
    #[case("x/y.z", "x-y-z")]
    fn escape_writes_styled_components_names(#[case] name: &str, #[case] escaped: &str) {
        assert_eq!(escape(name), escaped);
    }
}
