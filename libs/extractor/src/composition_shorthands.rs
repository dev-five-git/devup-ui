//! A shorthand written after a longhand it covers wins, as the later
//! declaration (#688), whatever order the stylesheet gives the two: the sheet
//! writes a longhand after every shorthand covering it, so the build drops the
//! earlier longhand's atom instead.
//!
//! Only what the build fully knows is dropped: both atoms are static and set
//! unconditionally, at the same selector, breakpoint, layer and style order. A
//! longhand or shorthand under a condition, or held by a variable, stays, and
//! the stylesheet's order (a longhand after its shorthands) decides between
//! them.

use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::style_selector::StyleSelector;

/// Whether the static `shorthand` replaces the static `longhand` written
/// before it: it covers the property where the two compete alike. A global
/// rule keeps both: it writes its declarations in the order written, where
/// the later one already wins
pub(crate) fn replaces(shorthand: &ExtractStaticStyle, longhand: &ExtractStaticStyle) -> bool {
    !matches!(shorthand.selector, Some(StyleSelector::Global(..)))
        && css::property_order::covers(&shorthand.property, &longhand.property)
        && shorthand.selector == longhand.selector
        && shorthand.level == longhand.level
        && shorthand.layer == longhand.layer
        && shorthand.style_order == longhand.style_order
}

/// The atom `prop` sets unconditionally to a static value
const fn static_atom<'p>(prop: &'p ExtractStyleProp<'_>) -> Option<&'p ExtractStaticStyle> {
    match prop {
        ExtractStyleProp::Static(ExtractStyleValue::Static(style)) => Some(style),
        _ => None,
    }
}

/// The static atoms of the declarations read so far, read from the last
/// declaration of a style object to its first
#[derive(Default)]
pub(crate) struct LaterDeclarations(Vec<ExtractStaticStyle>);

impl LaterDeclarations {
    /// The styles of a declaration written before every one read so far,
    /// without the longhands a shorthand among those covers
    pub(crate) fn before<'a>(
        &mut self,
        styles: Vec<ExtractStyleProp<'a>>,
    ) -> Vec<ExtractStyleProp<'a>> {
        let kept: Vec<ExtractStyleProp<'a>> = styles
            .into_iter()
            .filter_map(|prop| self.keep(prop))
            .collect();
        for prop in &kept {
            self.record(prop);
        }
        kept
    }

    /// `prop` without what a later shorthand replaces; the atoms of a
    /// responsive value apply unconditionally, each at its breakpoint
    fn keep<'a>(&self, prop: ExtractStyleProp<'a>) -> Option<ExtractStyleProp<'a>> {
        match prop {
            ExtractStyleProp::StaticArray(props) => Some(ExtractStyleProp::StaticArray(
                props
                    .into_iter()
                    .filter_map(|prop| self.keep(prop))
                    .collect(),
            )),
            prop => (!self.covers(&prop)).then_some(prop),
        }
    }

    fn record(&mut self, prop: &ExtractStyleProp<'_>) {
        match prop {
            ExtractStyleProp::StaticArray(props) => {
                for prop in props {
                    self.record(prop);
                }
            }
            prop => self.0.extend(static_atom(prop).cloned()),
        }
    }

    fn covers(&self, prop: &ExtractStyleProp<'_>) -> bool {
        static_atom(prop)
            .is_some_and(|longhand| self.0.iter().any(|shorthand| replaces(shorthand, longhand)))
    }
}

/// The styles of a style object's declarations, given in source order, without
/// the longhands a later shorthand covers
pub(crate) fn in_source_order<'a>(
    declarations: Vec<Vec<ExtractStyleProp<'a>>>,
) -> Vec<ExtractStyleProp<'a>> {
    let mut later = LaterDeclarations::default();
    let mut kept: Vec<Vec<ExtractStyleProp<'a>>> = declarations
        .into_iter()
        .rev()
        .map(|styles| later.before(styles))
        .collect();
    kept.reverse();
    kept.into_iter().flatten().collect()
}
