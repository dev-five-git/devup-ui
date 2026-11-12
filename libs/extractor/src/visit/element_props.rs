use crate::extractor::ExtractResult;
use crate::extractor::extract_style_from_expression::{
    LiteralHandling, extract_style_from_expression, extract_with_policy,
};
use crate::extractor::style_inputs::StyleInputs;
use crate::utils::{
    SPREAD_KEYS, STYLE_OBJECT, get_str_by_property_key, readable_code, unwrap_syntax_only,
};
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::{disassemble_property, is_special_property::is_special_property};
use oxc_allocator::{CloneIn, GetAllocator, TakeIn};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::{LogicalOperator, UnaryOperator};
use rustc_hash::FxHashSet;
use std::borrow::Cow;

#[cfg(test)]
#[path = "element_props_tests.rs"]
mod tests;

/// Prove the entire spread's own keys before capture replaces its syntax.
pub(crate) fn is_unknown_spread(argument: &Expression<'_>) -> bool {
    let mut pending = vec![argument];
    while let Some(value) = pending.pop() {
        let Expression::ObjectExpression(object) = unwrap_syntax_only(value) else {
            return true;
        };
        for property in &object.properties {
            match property {
                ObjectPropertyKind::SpreadProperty(spread) => pending.push(&spread.argument),
                ObjectPropertyKind::ObjectProperty(property) => {
                    if property.kind != PropertyKind::Init
                        || property.method
                        || get_str_by_property_key(&property.key).is_none()
                        || (!property.computed
                            && property
                                .key
                                .static_name()
                                .is_some_and(|key| key == "__proto__"))
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// What of a spread the build cannot tell apart from the styles it names,
/// which would then reach the element as props: where, the code, and why
pub(super) fn hidden_styles<'a>(
    inputs: StyleInputs<'_, 'a>,
    argument: &Expression<'a>,
) -> Option<(u32, String, &'static str)> {
    let mut styled = false;
    let mut unknown = None;
    let mut pending = vec![argument];
    while let Some(value) = pending.pop() {
        let value = unwrap_syntax_only(value);
        match value {
            Expression::ObjectExpression(object) if names_styles(inputs, object) => {
                if is_unknown_spread(value) {
                    return Some((value.span().start, readable_code(value), SPREAD_KEYS));
                }
                styled = true;
            }
            Expression::ObjectExpression(_) => {
                if is_unknown_spread(value) {
                    unknown.get_or_insert(value);
                }
            }
            // Either side may be what reaches the element, in the order written
            Expression::ConditionalExpression(branch) => {
                pending.extend([&branch.alternate, &branch.consequent]);
            }
            // A falsy left side is a primitive, which spreads nothing
            Expression::LogicalExpression(branch) if branch.operator == LogicalOperator::And => {
                pending.push(&branch.right);
            }
            Expression::LogicalExpression(branch) => {
                pending.extend([&branch.right, &branch.left]);
            }
            Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => {}
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void => {}
            Expression::Identifier(name)
                if matches!(name.name.as_str(), "undefined" | "NaN" | "Infinity") => {}
            // Any other side gives the styles the build reads from it, or
            // props only the running code knows
            value => {
                let mut copy = value.clone_in_with_semantic_ids(inputs.ast.allocator());
                let styles = extract_style_from_expression(
                    inputs,
                    None,
                    &mut copy,
                    0,
                    &None,
                    LiteralHandling::ExpandResponsiveThemeToken,
                )
                .styles;
                if styles
                    .iter()
                    .any(|style| matches!(style, ExtractStyleProp::Unreadable { .. }))
                {
                    unknown.get_or_insert(value);
                } else if !styles.is_empty() {
                    styled = true;
                }
            }
        }
    }
    unknown
        .filter(|_| styled)
        .map(|value| (value.span().start, readable_code(value), STYLE_OBJECT))
}

/// Whether the plain properties of `object` give styles
fn names_styles<'a>(inputs: StyleInputs<'_, 'a>, object: &ObjectExpression<'a>) -> bool {
    let mut known = object.clone_in_with_semantic_ids(inputs.ast.allocator());
    known.properties.retain(|property| {
        matches!(property, ObjectPropertyKind::ObjectProperty(property)
            if property.kind == PropertyKind::Init
                && !property.method
                && get_str_by_property_key(&property.key).is_some())
    });
    !extract_props(inputs, &mut known, &mut FxHashSet::default(), &mut |_| {})
        .styles
        .is_empty()
}

/// `on && { … }` spreads what `on ? { … } : null` does, as a falsy left side is
/// a primitive: write it so, for the conditional rules to read
pub(super) fn as_conditional<'a>(ast: &AstBuilder<'a>, value: &mut Expression<'a>) {
    let value = crate::utils::unwrap_syntax_only_mut(value);
    let guarded = matches!(value, Expression::LogicalExpression(branch)
        if branch.operator == LogicalOperator::And
            && matches!(unwrap_syntax_only(&branch.right), Expression::ObjectExpression(_)));
    if guarded && let Expression::LogicalExpression(branch) = value.take_in(ast) {
        let branch = branch.unbox();
        *value = Expression::new_conditional_expression(
            branch.span,
            branch.left,
            branch.right,
            Expression::new_null_literal(SPAN, ast),
            ast,
        );
    }
}

/// Conditional rules keep their original owner before any opaque-props snapshot.
/// Logical roots keep the existing raw-props snapshot contract.
pub(super) fn is_rule_branch(value: &Expression<'_>) -> bool {
    if !matches!(
        unwrap_syntax_only(value),
        Expression::ConditionalExpression(_)
    ) {
        return false;
    }
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        match unwrap_syntax_only(value) {
            Expression::ConditionalExpression(branch) => {
                pending.extend([&branch.consequent, &branch.alternate]);
            }
            Expression::LogicalExpression(branch) => {
                pending.extend([&branch.left, &branch.right]);
            }
            Expression::ObjectExpression(_) if is_unknown_spread(value) => return false,
            _ => {}
        }
    }
    true
}

/// A refused whole-rule leaf must reach the existing extractor with its authored span.
/// Reuse its inert-leaf policy; do not replace unknown branches with packet slots.
pub(super) fn unreadable_branch(value: &Expression<'_>) -> bool {
    match unwrap_syntax_only(value) {
        Expression::ConditionalExpression(branch) => {
            unreadable_branch(&branch.consequent) || unreadable_branch(&branch.alternate)
        }
        Expression::ObjectExpression(_)
        | Expression::StringLiteral(_)
        | Expression::TemplateLiteral(_) => false,
        value => !crate::extractor::extract_style_from_expression::unreadable(value)
            .styles
            .is_empty(),
    }
}

/// Whether style extraction left props for a selected own-data branch to pass on.
pub(super) fn has_runtime_props(value: &Expression<'_>) -> bool {
    match unwrap_syntax_only(value) {
        Expression::ObjectExpression(object) => !object.properties.is_empty(),
        Expression::ConditionalExpression(branch) => {
            has_runtime_props(&branch.consequent) || has_runtime_props(&branch.alternate)
        }
        Expression::LogicalExpression(branch) => has_runtime_props(&branch.right),
        _ => false,
    }
}

/// Keep native Emotion's flat-literal admission separate from Devup's recursive D1 proof.
pub(super) fn is_unknown_native_css_spread(argument: &Expression<'_>) -> bool {
    let Expression::ObjectExpression(object) = unwrap_syntax_only(argument) else {
        return true;
    };
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::SpreadProperty(_) => true,
        ObjectPropertyKind::ObjectProperty(property) => {
            get_str_by_property_key(&property.key).is_none()
                || property.kind != PropertyKind::Init
                || property.method
        }
    })
}

/// Flatten eligible spreads without dropping values before source-order capture.
pub(super) fn flatten_known<'a>(ast: &AstBuilder<'a>, object: &mut ObjectExpression<'a>) {
    let mut pending: Vec<_> =
        std::mem::replace(&mut object.properties, oxc_allocator::Vec::new_in(ast))
            .into_iter()
            .rev()
            .collect();
    while let Some(property) = pending.pop() {
        match property {
            ObjectPropertyKind::SpreadProperty(mut spread)
                if !is_unknown_spread(&spread.argument) =>
            {
                if let Expression::ObjectExpression(inner) =
                    crate::utils::unwrap_syntax_only_mut(&mut spread.argument)
                {
                    let properties =
                        std::mem::replace(&mut inner.properties, oxc_allocator::Vec::new_in(ast));
                    pending.extend(properties.into_iter().rev());
                }
            }
            property => object.properties.push(property),
        }
    }
}

pub(super) fn retains_target(
    style: &mut ExtractStyleProp<'_>,
    allowed: &FxHashSet<String>,
) -> bool {
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Static(value)) => {
            allowed.contains(value.property())
        }
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value)) => {
            allowed.contains(value.property())
        }
        ExtractStyleProp::StaticArray(styles) => {
            styles.retain_mut(|style| retains_target(style, allowed));
            !styles.is_empty()
        }
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => {
            for branch in [&mut *consequent, &mut *alternate] {
                if branch
                    .as_mut()
                    .is_some_and(|style| !retains_target(style, allowed))
                {
                    *branch = None;
                }
            }
            consequent.is_some() || alternate.is_some()
        }
        _ => true,
    }
}

/// Extract only authored properties; opaque spreads never enter the style extractor.
pub(super) fn extract_props<'a>(
    inputs: StyleInputs<'_, 'a>,
    object: &mut ObjectExpression<'a>,
    written: &mut FxHashSet<Cow<'a, str>>,
    prepare: &mut impl FnMut(&mut Expression<'a>),
) -> ExtractResult<'a> {
    let ast = inputs.ast;
    let mut result = ExtractResult::default();
    for index in (0..object.properties.len()).rev() {
        let ObjectPropertyKind::ObjectProperty(property) = &object.properties[index] else {
            continue;
        };
        let Some(key) = get_str_by_property_key(&property.key) else {
            continue;
        };
        if is_special_property(&key) {
            continue;
        }
        let names: Vec<_> = disassemble_property(&key).map(Cow::into_owned).collect();
        let allowed: FxHashSet<_> = names
            .iter()
            .filter(|name| !written.contains(name.as_str()))
            .cloned()
            .collect();
        let partial = allowed.len() < names.len();
        written.extend(names.into_iter().map(Cow::Owned));
        let property = object.properties.remove(index);
        if allowed.is_empty() {
            continue;
        }
        let mut single = Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_array_in([property], ast),
            ast,
        );
        let mut extracted = extract_with_policy::<false>(
            inputs,
            None,
            &mut single,
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
            prepare,
        );
        if partial {
            extracted
                .styles
                .retain_mut(|style| retains_target(style, &allowed));
        }
        result.styles.extend(extracted.styles);
        if !allowed.is_empty() {
            result.tag = result.tag.or(extracted.tag);
            result.style_order = result.style_order.or(extracted.style_order);
            result.style_vars = result.style_vars.or(extracted.style_vars);
            result.props = result.props.or(extracted.props);
        }
    }
    result
}
