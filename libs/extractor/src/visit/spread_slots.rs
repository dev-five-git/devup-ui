//! Styles an element writes before a spread the build cannot read. The spread
//! may carry the same key, which then wins: each such style becomes a rule that
//! reads a variable the element sets from the spreads, and keeps its own value
//! while no spread written after it has the key.

use super::DevupVisitor;
use crate::ExtractStyleProp;
use crate::extract_style::{
    extract_dynamic_style::ExtractDynamicStyle,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
    extract_style_value::ExtractStyleValue,
};
use crate::utils::{element_error, get_str_by_property_key, readable_code, unwrap_syntax_only};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};

/// Whether `object`, spread into props, may carry keys the build cannot see
fn object_is_unknown(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::SpreadProperty(_) => true,
        ObjectPropertyKind::ObjectProperty(property) => {
            get_str_by_property_key(&property.key).is_none()
                || property.kind != oxc_ast::ast::PropertyKind::Init
                || property.method
        }
    })
}

/// Whether `argument`, spread into props, may carry keys the build cannot see
pub(super) fn is_unknown_spread(argument: &Expression<'_>) -> bool {
    match unwrap_syntax_only(argument) {
        Expression::ObjectExpression(object) => object_is_unknown(object),
        _ => true,
    }
}

/// What an element writes under `key` before a spread, with how many unknown
/// spreads follow it and where it is written
pub(super) struct Overridden<'a> {
    pub key: String,
    pub offset: u32,
    pub spreads: usize,
    pub styles: Vec<ExtractStyleProp<'a>>,
}

/// What no stylesheet rule can give way to a spread
enum Unbindable {
    /// A rule under a selector: the spread's object replaces the whole object
    Selector,
    /// A value the build cannot place under one variable
    Shape,
}

/// Where the value of a written key comes from once spreads follow it: the
/// snapshots of the spreads written after it, in the order written
struct Slot<'s> {
    key: &'s str,
    sources: &'s [String],
}

impl Slot<'_> {
    /// The code giving what the last of the snapshots having `key` gives
    /// under it, as the spread assigned it (even `undefined`), or `absent`
    /// when none of them has it. A snapshot has no prototype, so `in` finds
    /// only the own enumerable properties the spread copied, and it holds the
    /// values already read, so no getter runs here
    fn read(&self, absent: &str) -> String {
        let key = serde_json::Value::from(self.key).to_string();
        let access = if self
            .key
            .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && self
                .key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            format!(".{}", self.key)
        } else {
            format!("[{key}]")
        };
        self.sources
            .iter()
            .fold(absent.to_string(), |otherwise, source| {
                format!("{key} in {source} ? {source}{access} : {otherwise}")
            })
    }
}

fn bind(style: &mut ExtractStyleProp<'_>, slot: &Slot<'_>) -> Result<(), Unbindable> {
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Static(value)) => {
            let first_value = value.theme_token_resolution() == ThemeTokenResolution::FirstValue
                && value.value().starts_with('$');
            if value.selector().is_some() {
                return Err(Unbindable::Selector);
            }
            if first_value || value.property() == "typography" {
                return Err(Unbindable::Shape);
            }
            *style = overridable(value, slot);
            Ok(())
        }
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value)) => {
            if value.selector().is_some() {
                return Err(Unbindable::Selector);
            }
            value.overridden_by(|written| slot.read(&format!("({written})")));
            Ok(())
        }
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Enum { .. }
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::MemberExpression { .. } => Err(Unbindable::Shape),
        ExtractStyleProp::StaticArray(styles) => {
            styles.iter_mut().try_for_each(|style| bind(style, slot))
        }
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => [consequent, alternate]
            .into_iter()
            .flatten()
            .try_for_each(|branch| bind(branch, slot)),
        ExtractStyleProp::Unreadable { .. } => Ok(()),
    }
}

fn overridable<'a>(value: &ExtractStaticStyle, slot: &Slot<'_>) -> ExtractStyleProp<'a> {
    ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
        ExtractDynamicStyle::overridable(value, &slot.read("void 0"), slot.key),
    ))
}

/// The name each of the unknown spreads `arguments` was snapshotted under, in
/// the order written
pub(super) fn sources<'e, 'a: 'e>(
    arguments: impl Iterator<Item = &'e Expression<'a>>,
) -> Vec<String> {
    arguments
        .map(|argument| readable_code(unwrap_syntax_only(argument)))
        .collect()
}

fn requirement(unbindable: &Unbindable, key: &str) -> String {
    match unbindable {
        Unbindable::Selector => format!(
            "a spread after it can replace the whole `{key}` object, which no stylesheet rule can follow: write the spread before `{key}`, or give the nested object through the spread yourself"
        ),
        Unbindable::Shape => format!(
            "a spread after it can replace it, which only a plain value of one CSS property can follow: write the spread before `{key}`, or give it a literal, a theme token or a runtime value"
        ),
    }
}

impl<'a> DevupVisitor<'a> {
    /// The styles `overridden` writes, each giving way to the spreads `sources`
    /// read after it; the ones no rule can give way are reported
    pub(super) fn bind_overridden(
        &mut self,
        element: &str,
        overridden: Vec<Overridden<'a>>,
        sources: &[String],
    ) -> Vec<ExtractStyleProp<'a>> {
        let mut bound = Vec::new();
        for Overridden {
            key,
            offset,
            spreads,
            mut styles,
        } in overridden
        {
            let slot = Slot {
                key: &key,
                sources: &sources[sources.len().saturating_sub(spreads)..],
            };
            match styles.iter_mut().try_for_each(|style| bind(style, &slot)) {
                Ok(()) => bound.extend(styles),
                Err(unbindable) => self.errors.push((
                    offset,
                    element_error(element, &key, &requirement(&unbindable, &key)),
                )),
            }
        }
        bound
    }
}

impl<'a> DevupVisitor<'a> {
    /// [`Self::bind_overridden`] for the props `object` of a `jsx()` call
    pub(super) fn bind_object_overridden(
        &mut self,
        element: &str,
        overridden: Vec<Overridden<'a>>,
        object: &ObjectExpression<'a>,
    ) -> Vec<ExtractStyleProp<'a>> {
        let spreads = object
            .properties
            .iter()
            .filter_map(|property| match property {
                ObjectPropertyKind::SpreadProperty(spread) => Some(&spread.argument),
                ObjectPropertyKind::ObjectProperty(_) => None,
            });
        let sources = sources(spreads);
        self.bind_overridden(element, overridden, &sources)
    }
}

mod take;
pub(super) use take::{take_known_overridden, take_overridden};
