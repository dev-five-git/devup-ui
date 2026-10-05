//! Taking the styles an object writes before spreads the build cannot read
//! out of it, one written key at a time.

use super::{Overridden, is_unknown_spread, object_is_unknown};
use crate::extractor::extract_style_from_expression::{
    LiteralHandling, extract_style_from_expression, flatten_spreads,
};
use crate::utils::get_str_by_property_key;
use css::disassemble_property;
use css::is_special_property::is_special_property;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectProperty, ObjectPropertyKind};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::FxHashSet;
use std::borrow::Cow;

/// The key `property` writes as a prop the build reads styles from
fn written_key(property: &ObjectProperty<'_>) -> Option<String> {
    get_str_by_property_key(&property.key)
        .filter(|key| {
            !is_special_property(key)
                && !matches!(key.as_ref(), "as" | "props" | "styleOrder" | "styleVars")
        })
        .map(Cow::into_owned)
}

/// Take what `object` writes under a key before an unknown spread out of it,
/// as the styles it gives, for [`super::DevupVisitor::bind_overridden`];
/// `trailing` unknown spreads follow `object` itself. A key written again
/// later is the one that counts, however a spread between them stands: the
/// earlier write is dropped, as it can never be seen
pub(in crate::visit) fn take_overridden<'a>(
    ast: &AstBuilder<'a>,
    object: &mut ObjectExpression<'a>,
    trailing: usize,
) -> Vec<Overridden<'a>> {
    flatten_spreads(ast, object);
    let mut spreads = trailing;
    let mut written: FxHashSet<String> = FxHashSet::default();
    let mut overridden = Vec::new();
    for index in (0..object.properties.len()).rev() {
        let key = match &object.properties[index] {
            ObjectPropertyKind::SpreadProperty(spread) => {
                spreads += usize::from(is_unknown_spread(&spread.argument));
                None
            }
            ObjectPropertyKind::ObjectProperty(property) => written_key(property),
        };
        let Some(key) = key else { continue };
        let names: Vec<String> = disassemble_property(&key).map(Cow::into_owned).collect();
        let shadowed = names.iter().all(|name| written.contains(name));
        written.extend(names);
        if spreads == 0 && !shadowed {
            continue;
        }
        let property = object.properties.remove(index);
        if shadowed {
            continue;
        }
        let offset = oxc_span::GetSpan::span(&property).start;
        let mut single = Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_array_in([property], ast),
            ast,
        );
        let styles = extract_style_from_expression(
            ast,
            None,
            &mut single,
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
        )
        .styles;
        overridden.push(Overridden {
            key,
            offset,
            spreads,
            styles,
        });
    }
    overridden
}

/// Take what the known object `object` writes, spread before `trailing`
/// unknown spreads, as [`take_overridden`] does for a `jsx()` call: the styles
/// each of its own keys gives, ready to read in the order a written attribute
/// would be
pub(in crate::visit) fn take_known_overridden<'a>(
    ast: &AstBuilder<'a>,
    object: &mut ObjectExpression<'a>,
    trailing: usize,
) -> Vec<Overridden<'a>> {
    if object_is_unknown(object) {
        return Vec::new();
    }
    let mut taken = take_overridden(ast, object, trailing);
    for written in &mut taken {
        written.styles.reverse();
    }
    taken
}
