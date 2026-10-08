use oxc_ast::ast::{
    Argument, CallExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind,
};
use oxc_span::GetSpan;

use super::{StylexFunction, StylexResolver, is_types_call};
use crate::utils::{
    build_time_error, get_string_by_literal_expression, get_string_by_property_key,
    js_number_literal, key_error, readable_argument, readable_code, unwrap_syntax_only,
};

enum Helper {
    FirstThatWorks,
    Include,
    Types,
}

fn helper(call: &CallExpression<'_>, resolver: StylexResolver<'_>) -> Option<Helper> {
    if is_types_call(&call.callee, resolver) {
        return Some(Helper::Types);
    }
    match resolver(&call.callee) {
        Some(StylexFunction::FirstThatWorks) => Some(Helper::FirstThatWorks),
        Some(StylexFunction::Include) => Some(Helper::Include),
        Some(StylexFunction::Types) => Some(Helper::Types),
        _ => None,
    }
}

fn call_code(call: &CallExpression<'_>) -> String {
    let arguments: Vec<_> = call.arguments.iter().map(readable_argument).collect();
    format!("{}({})", readable_code(&call.callee), arguments.join(", "))
}

/// Validate helper arity before an enclosing API consumes its arguments.
pub(crate) fn validate_helper_call(
    call: &CallExpression<'_>,
    resolver: StylexResolver<'_>,
) -> Result<(), (u32, String)> {
    let requirement = match helper(call, resolver) {
        Some(Helper::Include)
            if call.arguments.len() != 1
                || call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_none() =>
        {
            Some((
                "stylex.include",
                "it requires exactly one namespace argument; pass one namespace such as `styles.base`",
            ))
        }
        Some(Helper::Types)
            if call.arguments.is_empty()
                || call
                    .arguments
                    .first()
                    .and_then(Argument::as_expression)
                    .is_none() =>
        {
            Some((
                "stylex.types",
                "a types.* wrapper requires a value argument; pass one value without spreading arguments",
            ))
        }
        Some(Helper::Types)
            if call.arguments.iter().skip(1).any(|argument| {
                argument
                    .as_expression()
                    .is_none_or(|value| !static_extra(value))
            }) =>
        {
            Some((
                "stylex.types",
                "extra arguments are evaluated before being ignored; use statically readable values without spreads or runtime side effects",
            ))
        }
        _ => None,
    };
    match requirement {
        Some((api, requirement)) => Err((
            call.span.start,
            build_time_error(api, &call_code(call), requirement),
        )),
        None => Ok(()),
    }
}

fn static_extra(value: &Expression<'_>) -> bool {
    let value = unwrap_syntax_only(value);
    if js_number_literal(value).is_some() || get_string_by_literal_expression(value).is_some() {
        return true;
    }
    match value {
        Expression::NullLiteral(_) => true,
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if !property.method && property.kind == PropertyKind::Init
                    && get_string_by_property_key(&property.key).is_some()
                    && static_extra(&property.value))
        }),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(static_extra)),
        _ => false,
    }
}

/// Invoke only on calls left after enclosing APIs have consumed their helpers.
pub(crate) fn unconsumed_helper_error(
    call: &CallExpression<'_>,
    resolver: StylexResolver<'_>,
) -> Option<(u32, String)> {
    let (api, requirement) = match helper(call, resolver)? {
        Helper::FirstThatWorks => (
            "stylex.firstThatWorks",
            "this helper is outside a consumed style value; move it into a property value inside `stylex.create()`",
        ),
        Helper::Include => (
            "stylex.include",
            "this helper is outside a consumed namespace spread; use `...stylex.include(styles.base)` inside a `stylex.create()` namespace",
        ),
        Helper::Types => (
            "stylex.types",
            "this wrapper is outside a consumed value; move it into a supported value inside `stylex.create()`, `stylex.defineVars()` or `stylex.createTheme()`",
        ),
    };
    Some((
        call.span.start,
        build_time_error(api, &call_code(call), requirement),
    ))
}

/// Check at-rule spelling without narrowing the caller's other condition forms.
pub(crate) fn validate_at_rule_condition(
    key: &PropertyKey<'_>,
    api: &str,
) -> Result<(), (u32, String)> {
    let Some(name) = get_string_by_property_key(key) else {
        return Err(key_error(api, key));
    };
    if name.starts_with('@') && css::at_rule::split_at_rule_key(&name).is_none() {
        return Err((
            key.span().start,
            build_time_error(
                api,
                &name,
                "an at-rule condition requires an exact @media, @supports or @container token and a nonempty query; use e.g. `@media (min-width: 600px)`",
            ),
        ));
    }
    Ok(())
}

/// Contract placeholders are flat null/string leaves, not variable values.
pub(crate) fn validate_contract_placeholder(
    key_path: &str,
    value: &Expression<'_>,
) -> Result<(), (u32, String)> {
    match unwrap_syntax_only(value) {
        Expression::NullLiteral(_) | Expression::StringLiteral(_) => Ok(()),
        _ => Err((
            value.span().start,
            build_time_error(
                "stylex.createThemeContract",
                &readable_code(value),
                &format!(
                    "placeholder at `{key_path}` must belong to a static flat null/string object; flatten nested keys (e.g. `paletteText`), use null or a string placeholder, and set values in `stylex.createTheme()`"
                ),
            ),
        )),
    }
}

#[cfg(test)]
mod tests;
