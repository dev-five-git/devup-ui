//! Closed compatibility policy for declarations with no unprefixed implementation.

use crate::{
    ExtractStyleProp,
    utils::{Unused, fixed_value, runtime_value, unreadable_styles},
};
use css::utils::to_kebab_case;

mod calls;
mod objects;
mod origin;
mod strings;
pub(crate) use origin::{evaluated_calls, map_evaluated};
mod text;
pub(crate) use calls::instrument;
pub(crate) use objects::authored_errors;
pub(crate) use objects::{ObjectKind, evaluated_errors};
pub(crate) const ERROR_CHANNEL: &str = "DEVUP_DEAD_DECLARATION\n";

pub(crate) fn terminal_error(message: &str) -> bool {
    message
        .split_once(" at build time: ")
        .is_some_and(|(_, requirement)| {
            requirement == crate::utils::RESPONSIVE_ARRAY
                || requirement.starts_with("unprefixed ")
                || requirement.starts_with("obsolete scroll-snap-")
        })
}
pub(crate) use text::{expression_errors, template_errors};

/// The replacement is advice, never an automatic alias or rewrite.
pub(crate) fn requirement(name: &str) -> Option<&'static str> {
    let canonical = if name.contains('-') {
        std::borrow::Cow::Owned(name.to_ascii_lowercase())
    } else {
        to_kebab_case(name)
    };
    Some(match canonical.as_ref() {
        "box-align" => {
            "unprefixed box-align has no browser implementation; use display: flex and align-items"
        }
        "box-pack" => {
            "unprefixed box-pack has no browser implementation; use display: flex and justify-content"
        }
        "box-flex" => "unprefixed box-flex has no browser implementation; use flex-grow",
        "box-flex-group" => {
            "unprefixed box-flex-group has no browser implementation; use flex-grow on modern flex items"
        }
        "box-orient" => "unprefixed box-orient has no browser implementation; use flex-direction",
        "box-ordinal-group" => {
            "unprefixed box-ordinal-group has no browser implementation; use order"
        }
        "box-direction" => {
            "unprefixed box-direction has no browser implementation; use flex-direction with row-reverse or column-reverse"
        }
        "box-lines" => "unprefixed box-lines has no browser implementation; use flex-wrap",
        "flex-order" => "unprefixed flex-order is an obsolete flexbox draft property; use order",
        "flex-positive" => {
            "unprefixed flex-positive is an obsolete flexbox draft property; use flex-grow"
        }
        "flex-negative" => {
            "unprefixed flex-negative is an obsolete flexbox draft property; use flex-shrink"
        }
        "flex-preferred-size" => {
            "unprefixed flex-preferred-size is an obsolete flexbox draft property; use flex-basis"
        }
        "scroll-snap-points-x" => {
            "obsolete scroll-snap-points-x is no longer implemented; use scroll-snap-type on the container and scroll-snap-align on its children"
        }
        "scroll-snap-points-y" => {
            "obsolete scroll-snap-points-y is no longer implemented; use scroll-snap-type on the container and scroll-snap-align on its children"
        }
        "scroll-snap-coordinate" => {
            "obsolete scroll-snap-coordinate is no longer implemented; use scroll-snap-align on the snap target"
        }
        "scroll-snap-destination" => {
            "obsolete scroll-snap-destination is no longer implemented; use scroll-padding on the snap container"
        }
        "scroll-snap-type-x" => {
            "obsolete scroll-snap-type-x is no longer implemented; use scroll-snap-type with an x axis"
        }
        "scroll-snap-type-y" => {
            "obsolete scroll-snap-type-y is no longer implemented; use scroll-snap-type with a y axis"
        }
        _ => return None,
    })
}

/// Reject a declaration at its authored key, not at its value or containing call.
pub(crate) fn declaration_error<'a>(name: &str, offset: u32) -> Option<ExtractStyleProp<'a>> {
    Some(ExtractStyleProp::Unreadable {
        offset,
        code: name.to_string(),
        prop: false,
        requirement: Some(requirement(name)?),
    })
}

pub(crate) fn authored_requirement(name: &str) -> Option<&'static str> {
    match css::disassemble_property(name) {
        css::DisassembleProperty::Custom(mut properties) => {
            properties.find_map(|property| requirement(&property))
        }
        css::DisassembleProperty::Mapped(mut properties) => {
            properties.find_map(|property| requirement(property))
        }
        css::DisassembleProperty::Fallback(_) => requirement(name),
    }
}

pub(crate) fn authored_declaration_error<'a>(
    name: &str,
    offset: u32,
) -> Option<ExtractStyleProp<'a>> {
    Some(ExtractStyleProp::Unreadable {
        offset,
        code: name.to_string(),
        prop: false,
        requirement: Some(authored_requirement(name)?),
    })
}

/// Retain the stored location while preserving the existing runtime/fixed-value fallback.
pub(crate) fn located_value(
    styles: &[ExtractStyleProp<'_>],
    fallback: u32,
    fixed: bool,
) -> Option<(u32, Unused)> {
    let mut found = Vec::new();
    unreadable_styles(styles, true, &mut found);
    match found.into_iter().next() {
        Some((offset, code, requirement)) => Some((
            if offset == 0 { fallback } else { offset },
            (code, requirement),
        )),
        None => if fixed {
            fixed_value(styles)
        } else {
            runtime_value(styles)
        }
        .map(|value| (fallback, value)),
    }
}
