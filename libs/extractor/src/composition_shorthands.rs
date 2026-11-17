//! A shorthand set across a boundary after a longhand it covers wins, as the
//! later style (#688), whatever order the stylesheet gives the two: the sheet
//! writes a longhand after every shorthand covering it, so the build drops the
//! earlier longhand's atom instead.
//!
//! The boundaries are the ones that order styles: the arguments of `css(a, b)`,
//! a spread among JSX attributes or object properties, and styled composition
//! or extension. Declarations of one level, the keys of one object literal or
//! the JSX attributes between two spreads, are not ordered by where they are
//! written, so a shorthand there drops no longhand beside it: in
//! `css({ borderTopWidth: 2, borderWidth: 1 })` both stay and the stylesheet's
//! order (a longhand after its shorthands) lets `border-top-width` win.
//!
//! Only what the build fully knows is dropped: both atoms are static and set
//! unconditionally, at the same selector, breakpoint, layer and style order. A
//! longhand or shorthand under a condition, or held by a variable, stays, and
//! the stylesheet's order decides between them.

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
/// declaration to its first
#[derive(Default)]
struct LaterDeclarations(Vec<ExtractStaticStyle>);

impl LaterDeclarations {
    /// `styles` without the longhands a shorthand read so far covers
    fn without_covered<'a>(&self, styles: Vec<ExtractStyleProp<'a>>) -> Vec<ExtractStyleProp<'a>> {
        styles
            .into_iter()
            .filter_map(|prop| self.keep(prop))
            .collect()
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

/// The static atoms of the JSX attributes or object properties read so far,
/// read from the last to the first.
///
/// The attributes or properties between two spreads form a run of one level:
/// a shorthand in a run replaces no longhand of the same run, and the
/// stylesheet's order (a longhand after its shorthands) decides between them.
/// For JSX this also keeps a run the lint sorts by name
/// (`perfectionist/sort-jsx-props`, `backgroundClip` before `bg`) as written.
/// A spread keeps its place among the runs, so a shorthand in a later run or
/// spread still replaces an earlier longhand. An object spread the build
/// inlines keeps its properties in runs of their own, and the properties of a
/// merge of several style objects, `css(a, b)`, run per object they were
/// written in
#[derive(Default)]
pub(crate) struct LaterRuns {
    /// The atoms of the runs and spreads after the run being read
    later: LaterDeclarations,
    /// The atoms of the run being read
    run: LaterDeclarations,
    /// Where the run being read was written
    key: RunKey,
}

/// Where a property was written: the merged style object, when a call merged
/// several, and the run between spreads within it
pub(crate) type RunKey = (Option<usize>, usize);

impl LaterRuns {
    /// The styles of an attribute or property written at `key`, before every
    /// one read so far, without the longhands a shorthand of a later run or
    /// spread covers
    pub(crate) fn member<'a>(
        &mut self,
        key: RunKey,
        styles: Vec<ExtractStyleProp<'a>>,
    ) -> Vec<ExtractStyleProp<'a>> {
        if key != self.key {
            self.end_run();
            self.key = key;
        }
        let kept = self.later.without_covered(styles);
        for prop in &kept {
            self.run.record(prop);
        }
        kept
    }

    /// A spread, whatever the build reads of it, ends the run being read
    pub(crate) fn end_run(&mut self) {
        self.later.0.append(&mut self.run.0);
    }

    /// The styles of a spread written before every attribute or property read
    /// so far, without the longhands a later shorthand covers. It ends the run
    /// being read
    pub(crate) fn spread<'a>(
        &mut self,
        styles: Vec<ExtractStyleProp<'a>>,
    ) -> Vec<ExtractStyleProp<'a>> {
        self.end_run();
        let kept = self.later.without_covered(styles);
        for prop in &kept {
            self.later.record(prop);
        }
        kept
    }
}

/// The styles of one property or spread of a style object
pub(crate) enum Part<'a> {
    /// A property, with where it was written
    Property(RunKey, Vec<ExtractStyleProp<'a>>),
    Spread(Vec<ExtractStyleProp<'a>>),
}

/// The styles of a style object's properties and spreads, given in source
/// order, without the longhands a shorthand across a spread covers
pub(crate) fn in_source_order<'a>(parts: Vec<Part<'a>>) -> Vec<ExtractStyleProp<'a>> {
    let mut later = LaterRuns::default();
    let mut kept: Vec<Vec<ExtractStyleProp<'a>>> = parts
        .into_iter()
        .rev()
        .map(|part| match part {
            Part::Property(key, styles) => later.member(key, styles),
            Part::Spread(styles) => later.spread(styles),
        })
        .collect();
    kept.reverse();
    kept.into_iter().flatten().collect()
}
