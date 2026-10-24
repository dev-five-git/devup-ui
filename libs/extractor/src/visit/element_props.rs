use crate::extractor::ExtractResult;
use crate::extractor::extract_style_from_expression::{LiteralHandling, extract_with_policy};
use crate::utils::{get_str_by_property_key, unwrap_syntax_only};
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::{disassemble_property, is_special_property::is_special_property};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::FxHashSet;
use std::borrow::Cow;

#[cfg(test)]
#[path = "element_props_tests.rs"]
mod tests;

/// Prove the entire spread's own keys before capture replaces its syntax.
pub(super) fn is_unknown_spread(argument: &Expression<'_>) -> bool {
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
    ast: &AstBuilder<'a>,
    object: &mut ObjectExpression<'a>,
    written: &mut FxHashSet<Cow<'a, str>>,
    prepare: &mut impl FnMut(&mut Expression<'a>),
) -> ExtractResult<'a> {
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
            ast,
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
