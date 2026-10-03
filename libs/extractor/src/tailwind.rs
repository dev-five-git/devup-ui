//! Tailwind CSS classes in a `className`, compiled to devup-ui styles
//!
//! A class compiles only when every part of it is understood: its variants
//! (`md:`, `hover:`, `data-[state=open]:`, `[&>*]:` …) and its utility, which
//! becomes the declarations Tailwind CSS v4 writes for it. Any other class is
//! left as written, for whatever else defines it.

// The nested if-let pattern is intentional for readability in parsing code.
// Using if-let chains would make the code harder to read and modify.
#![allow(clippy::collapsible_if)]

use std::borrow::Cow;
use std::collections::BTreeSet;

use css::tailwind_definitions::{custom_variant, theme_value, theme_variable};
use css::{
    at_rule::split_at_rule_key,
    style_selector::{AtRuleKind, StyleSelector},
};
use phf::{phf_map, phf_set};

use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::tailwind_children::{CHILDREN, children_utility};
use crate::tailwind_color::color_value;
use crate::tailwind_custom::{custom_declarations, functional_declarations};
use crate::tailwind_effects::effect_utility;
use crate::tailwind_misc::{misc_utility, nested_utility};
use crate::tailwind_motion::{keyframes_of, motion_utility};
use crate::tailwind_theme::{reads_unsupported_theme, scaled, themed};

/// A declaration: property and value
pub type Declaration = (Cow<'static, str>, Cow<'static, str>);

/// Breakpoint level of each responsive variant; the breakpoints themselves
/// come from the devup theme
static RESPONSIVE_PREFIX_MAP: phf::Map<&'static str, u8> = phf_map! {
    "sm" => 1,
    "md" => 2,
    "lg" => 3,
    "xl" => 4,
    "2xl" => 5,
};

/// Media query Tailwind guards hover styles with, so that a tap on a touch
/// screen does not leave them applied
const HOVER_QUERY: &str = "(hover: hover)";

/// Pseudo-class variants, which `group-*`, `peer-*` and `has-*` also take
static STATE_VARIANTS: phf::Map<&'static str, &'static str> = phf_map! {
    "first" => "&:first-child",
    "last" => "&:last-child",
    "only" => "&:only-child",
    "odd" => "&:nth-child(odd)",
    "even" => "&:nth-child(even)",
    "first-of-type" => "&:first-of-type",
    "last-of-type" => "&:last-of-type",
    "only-of-type" => "&:only-of-type",
    "visited" => "&:visited",
    "target" => "&:target",
    "open" => "&:is([open], :popover-open, :open)",
    "default" => "&:default",
    "checked" => "&:checked",
    "indeterminate" => "&:indeterminate",
    "placeholder-shown" => "&:placeholder-shown",
    "autofill" => "&:autofill",
    "optional" => "&:optional",
    "required" => "&:required",
    "valid" => "&:valid",
    "invalid" => "&:invalid",
    "user-valid" => "&:user-valid",
    "user-invalid" => "&:user-invalid",
    "in-range" => "&:in-range",
    "out-of-range" => "&:out-of-range",
    "read-only" => "&:read-only",
    "empty" => "&:empty",
    "focus-within" => "&:focus-within",
    "hover" => "&:hover",
    "focus" => "&:focus",
    "focus-visible" => "&:focus-visible",
    "active" => "&:active",
    "enabled" => "&:enabled",
    "disabled" => "&:disabled",
    "inert" => "&:is([inert], [inert] *)",
};

/// Pseudo-element variants. A selector list holding a pseudo-element some
/// browser does not know is dropped whole there, so each gets a rule of its own.
static PSEUDO_ELEMENT_VARIANTS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "before" => &["&::before"],
    "after" => &["&::after"],
    "placeholder" => &["&::placeholder"],
    "file" => &["&::file-selector-button"],
    "backdrop" => &["&::backdrop"],
    "first-letter" => &["&::first-letter"],
    "first-line" => &["&::first-line"],
    "details-content" => &["&::details-content"],
    "selection" => &["& *::selection", "&::selection"],
    "marker" => &[
        "& *::marker",
        "&::marker",
        "& *::-webkit-details-marker",
        "&::-webkit-details-marker",
    ],
};

/// Other selector variants
static SELECTOR_VARIANTS: phf::Map<&'static str, &'static str> = phf_map! {
    // The devup theme picks the color scheme: `prefers-color-scheme` until a
    // theme is set on the root
    "dark" => ":root[data-theme=dark] &",
    "rtl" => "&:where(:dir(rtl), [dir=\"rtl\"], [dir=\"rtl\"] *)",
    "ltr" => "&:where(:dir(ltr), [dir=\"ltr\"], [dir=\"ltr\"] *)",
    "*" => ":is(& > *)",
    "**" => ":is(& *)",
};

/// Media query variants
static MEDIA_VARIANTS: phf::Map<&'static str, &'static str> = phf_map! {
    "motion-safe" => "(prefers-reduced-motion: no-preference)",
    "motion-reduce" => "(prefers-reduced-motion: reduce)",
    "contrast-more" => "(prefers-contrast: more)",
    "contrast-less" => "(prefers-contrast: less)",
    "portrait" => "(orientation: portrait)",
    "landscape" => "(orientation: landscape)",
    "print" => "print",
    "forced-colors" => "(forced-colors: active)",
    "inverted-colors" => "(inverted-colors: inverted)",
    "noscript" => "(scripting: none)",
    "pointer-fine" => "(pointer: fine)",
    "pointer-coarse" => "(pointer: coarse)",
    "pointer-none" => "(pointer: none)",
    "any-pointer-fine" => "(any-pointer: fine)",
    "any-pointer-coarse" => "(any-pointer: coarse)",
    "any-pointer-none" => "(any-pointer: none)",
};

/// The boolean ARIA attributes `aria-*` variants check for `true`
static ARIA_VARIANTS: phf::Set<&'static str> = phf_set! {
    "busy",
    "checked",
    "disabled",
    "expanded",
    "hidden",
    "pressed",
    "readonly",
    "required",
    "selected",
};

/// `@property` rule of a custom property: with an initial value, or without
macro_rules! property {
    ($name:literal, $syntax:literal, $initial:literal) => {
        (
            $name,
            concat!(
                "@property ",
                $name,
                "{syntax:\"",
                $syntax,
                "\";inherits:false;initial-value:",
                $initial,
                "}"
            ),
        )
    };
    ($name:literal) => {
        (
            $name,
            concat!("@property ", $name, "{syntax:\"*\";inherits:false}"),
        )
    };
}

/// The custom properties Tailwind registers, so that each element starts from
/// their initial value instead of inheriting its parent's
static PROPERTIES: &[(&str, &str)] = &[
    property!("--tw-translate-x", "*", "0"),
    property!("--tw-translate-y", "*", "0"),
    property!("--tw-translate-z", "*", "0"),
    property!("--tw-ordinal"),
    property!("--tw-slashed-zero"),
    property!("--tw-numeric-figure"),
    property!("--tw-numeric-spacing"),
    property!("--tw-numeric-fraction"),
    property!("--tw-contain-size"),
    property!("--tw-contain-layout"),
    property!("--tw-contain-paint"),
    property!("--tw-contain-style"),
    property!("--tw-scale-x", "*", "1"),
    property!("--tw-scale-y", "*", "1"),
    property!("--tw-scale-z", "*", "1"),
    property!("--tw-rotate-x"),
    property!("--tw-rotate-y"),
    property!("--tw-rotate-z"),
    property!("--tw-skew-x"),
    property!("--tw-skew-y"),
    property!("--tw-leading"),
    property!("--tw-content", "*", "\"\""),
    property!("--tw-scroll-snap-strictness", "*", "proximity"),
    property!("--tw-blur"),
    property!("--tw-brightness"),
    property!("--tw-contrast"),
    property!("--tw-grayscale"),
    property!("--tw-hue-rotate"),
    property!("--tw-invert"),
    property!("--tw-saturate"),
    property!("--tw-sepia"),
    property!("--tw-drop-shadow"),
    property!("--tw-backdrop-blur"),
    property!("--tw-backdrop-brightness"),
    property!("--tw-backdrop-contrast"),
    property!("--tw-backdrop-grayscale"),
    property!("--tw-backdrop-hue-rotate"),
    property!("--tw-backdrop-invert"),
    property!("--tw-backdrop-opacity"),
    property!("--tw-backdrop-saturate"),
    property!("--tw-backdrop-sepia"),
    property!("--tw-gradient-position"),
    property!("--tw-gradient-from", "<color>", "#0000"),
    property!("--tw-gradient-via", "<color>", "#0000"),
    property!("--tw-gradient-to", "<color>", "#0000"),
    property!("--tw-gradient-stops"),
    property!("--tw-gradient-via-stops"),
    property!("--tw-gradient-from-position", "<length-percentage>", "0%"),
    property!("--tw-gradient-via-position", "<length-percentage>", "50%"),
    property!("--tw-gradient-to-position", "<length-percentage>", "100%"),
    property!("--tw-space-x-reverse", "*", "0"),
    property!("--tw-space-y-reverse", "*", "0"),
    property!("--tw-divide-x-reverse", "*", "0"),
    property!("--tw-divide-y-reverse", "*", "0"),
    property!("--tw-border-style", "*", "solid"),
    property!("--tw-outline-style", "*", "solid"),
    property!("--tw-border-spacing-x", "<length>", "0"),
    property!("--tw-border-spacing-y", "<length>", "0"),
    property!("--tw-duration"),
    property!("--tw-ease"),
    property!("--tw-shadow", "*", "0 0 #0000"),
    property!("--tw-shadow-color"),
    property!("--tw-shadow-alpha", "<percentage>", "100%"),
    property!("--tw-text-shadow-color"),
    property!("--tw-text-shadow-alpha", "<percentage>", "100%"),
    property!("--tw-drop-shadow-color"),
    property!("--tw-drop-shadow-alpha", "<percentage>", "100%"),
    property!("--tw-drop-shadow-size"),
    property!("--tw-inset-shadow", "*", "0 0 #0000"),
    property!("--tw-inset-shadow-color"),
    property!("--tw-inset-shadow-alpha", "<percentage>", "100%"),
    property!("--tw-ring-color"),
    property!("--tw-ring-inset"),
    property!("--tw-ring-offset-width", "<length>", "0px"),
    property!("--tw-ring-offset-color", "*", "#fff"),
    property!("--tw-ring-offset-shadow", "*", "0 0 #0000"),
    property!("--tw-ring-shadow", "*", "0 0 #0000"),
    property!("--tw-inset-ring-color"),
    property!("--tw-inset-ring-shadow", "*", "0 0 #0000"),
];

/// Whether `value` reads the custom property `name`, whole and not as the
/// start of a longer name
fn reads(value: &str, name: &str) -> bool {
    value.match_indices(name).any(|(start, _)| {
        !value[start + name.len()..]
            .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}
/// The stylesheet keeps the rules of [`TailwindClass::rules`] under this name, which no source
/// file has, so that no file's update removes them
pub const PROPERTY_RULES_FILE: &str = "@devup-ui/tailwind";

/// What one variant adds to the condition a class applies under
#[derive(Default)]
struct Variant {
    /// Breakpoint level
    level: u8,
    /// Selectors the element matches, any one of them; `&` is the element
    selectors: Vec<Cow<'static, str>>,
    /// At-rule the rule goes in
    at_rule: Option<(AtRuleKind, Cow<'static, str>)>,
    /// `::before`/`::after`, which only render with `content`
    content: bool,
}

impl Variant {
    fn selector(selector: impl Into<Cow<'static, str>>) -> Self {
        Self {
            selectors: vec![selector.into()],
            ..Self::default()
        }
    }

    fn at_rule(kind: AtRuleKind, query: impl Into<Cow<'static, str>>) -> Self {
        Self {
            at_rule: Some((kind, query.into())),
            ..Self::default()
        }
    }

    /// A state the element is in, hovering only on devices that can hover
    fn state(selector: Cow<'static, str>, hover: bool) -> Self {
        Self {
            selectors: vec![selector],
            at_rule: hover.then_some((AtRuleKind::Media, Cow::Borrowed(HOVER_QUERY))),
            ..Self::default()
        }
    }
}

/// A Tailwind class, compiled
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailwindClass {
    /// Breakpoint level
    pub level: u8,
    /// The conditions the declarations apply under, one rule each
    pub conditions: Vec<Option<StyleSelector>>,
    /// The declarations, in the order Tailwind writes them
    pub declarations: Vec<Declaration>,
    /// Declarations that go in an at-rule of their own inside the rule, with
    /// the conditions of each
    pub nested: Vec<(Vec<Option<StyleSelector>>, Vec<Declaration>)>,
}

impl TailwindClass {
    /// Every declaration under every condition
    pub fn styles(&self) -> impl Iterator<Item = ExtractStaticStyle> + '_ {
        let groups = std::iter::once((&self.conditions, &self.declarations)).chain(
            self.nested
                .iter()
                .map(|(conditions, declarations)| (conditions, declarations)),
        );
        groups.flat_map(move |(conditions, declarations)| {
            conditions.iter().flat_map(move |condition| {
                declarations.iter().map(move |(property, value)| {
                    ExtractStaticStyle::new(property, value, self.level, condition.clone())
                })
            })
        })
    }

    /// Every declaration the class has
    fn all_declarations(&self) -> impl Iterator<Item = &Declaration> {
        self.declarations.iter().chain(
            self.nested
                .iter()
                .flat_map(|(_, declarations)| declarations),
        )
    }

    /// The global rules the declarations need: the @property rules of the
    /// custom properties they set or read, and the @keyframes they name
    #[must_use]
    pub fn rules(&self) -> Vec<Cow<'static, str>> {
        let mut rules: Vec<Cow<'static, str>> = PROPERTIES
            .iter()
            .filter(|(name, _)| {
                self.all_declarations()
                    .any(|(property, value)| property == name || reads(value, name))
            })
            .map(|&(_, rule)| Cow::Borrowed(rule))
            .collect();
        let all: Vec<Declaration> = self.all_declarations().cloned().collect();
        rules.extend(keyframes_of(&all));
        rules
    }
}

/// The widths of the containers of the default theme
fn container_width(key: &str) -> Option<&'static str> {
    Some(match key {
        "3xs" => "16rem",
        "2xs" => "18rem",
        "xs" => "20rem",
        "sm" => "24rem",
        "md" => "28rem",
        "lg" => "32rem",
        "xl" => "36rem",
        "2xl" => "42rem",
        "3xl" => "48rem",
        "4xl" => "56rem",
        "5xl" => "64rem",
        "6xl" => "72rem",
        "7xl" => "80rem",
        _ => return None,
    })
}

/// `@md:`, `@max-md:`, `@min-[400px]:` and `@md/main:`, which style an element
/// by the width of its container, or of the container named `main`
fn container_variant(name: &str) -> Option<Variant> {
    let rest = name.strip_prefix('@')?;
    let parts = split_top_level(rest, '/');
    let (query, container) = match parts.as_slice() {
        [query] => (*query, None),
        [query, container] if !container.is_empty() => (*query, Some(*container)),
        _ => return None,
    };
    let (comparison, key) = if let Some(key) = query.strip_prefix("max-") {
        ("<", key)
    } else {
        (">=", query.strip_prefix("min-").unwrap_or(query))
    };
    let width = if let Some(value) = bracketed(key) {
        decode_arbitrary_value(value)
    } else {
        themed("container", key, container_width)?.into_owned()
    };
    if width.is_empty() || width.contains("var(") {
        return None;
    }
    let condition = format!("(width {comparison} {width})");
    Some(Variant::at_rule(
        AtRuleKind::Container,
        container.map_or_else(
            || condition.clone(),
            |container| format!("{container} {condition}"),
        ),
    ))
}

/// The variant a prefix such as `hover` or `md` stands for
fn variant(name: &str) -> Option<Variant> {
    if let Some(custom) = custom_variant(name) {
        return Some(Variant {
            selectors: custom.selectors.into_iter().map(Cow::Owned).collect(),
            at_rule: custom
                .at_rule
                .map(|(kind, query)| (kind, Cow::Owned(query))),
            ..Variant::default()
        });
    }
    if let Some(variant) = container_variant(name) {
        return Some(variant);
    }
    if let Some(&level) = RESPONSIVE_PREFIX_MAP.get(name) {
        return Some(Variant {
            level,
            ..Variant::default()
        });
    }
    if let Some(width) = theme_value("breakpoint", name) {
        return Some(Variant::at_rule(
            AtRuleKind::Media,
            format!("(min-width:{width})"),
        ));
    }
    if let Some(&query) = MEDIA_VARIANTS.get(name) {
        return Some(Variant::at_rule(AtRuleKind::Media, query));
    }
    if let Some(&selectors) = PSEUDO_ELEMENT_VARIANTS.get(name) {
        return Some(Variant {
            selectors: selectors.iter().copied().map(Cow::Borrowed).collect(),
            content: matches!(name, "before" | "after"),
            ..Variant::default()
        });
    }
    if let Some(&selector) = SELECTOR_VARIANTS.get(name) {
        return Some(Variant::selector(selector));
    }
    if let Some(selector) = state_selector(name) {
        return Some(Variant::state(selector, name == "hover"));
    }
    if let Some(state) = name.strip_prefix("group-") {
        return relation_variant(state, "group", " *");
    }
    if let Some(state) = name.strip_prefix("peer-") {
        return relation_variant(state, "peer", " ~ *");
    }
    if let Some(condition) = name.strip_prefix("supports-") {
        return supports_query(bracketed(condition)?)
            .map(|query| Variant::at_rule(AtRuleKind::Supports, query));
    }
    arbitrary_variant(bracketed(name)?)
}

/// The selector of a state the element is in, which `group-*`, `peer-*` and
/// `has-*` look for on another element
fn state_selector(name: &str) -> Option<Cow<'static, str>> {
    if let Some(&selector) = STATE_VARIANTS.get(name) {
        return Some(Cow::Borrowed(selector));
    }
    if let Some(attribute) = name.strip_prefix("aria-") {
        if ARIA_VARIANTS.contains(attribute) {
            return Some(Cow::Owned(format!("&[aria-{attribute}=\"true\"]")));
        }
        return attribute_selector("aria-", bracketed(attribute)?);
    }
    if let Some(attribute) = name.strip_prefix("data-") {
        return match bracketed(attribute) {
            Some(inner) => attribute_selector("data-", inner),
            None => is_name(attribute).then(|| Cow::Owned(format!("&[data-{attribute}]"))),
        };
    }
    if let Some(position) = name.strip_prefix("nth-") {
        return nth_selector(position);
    }
    if let Some(inner) = name.strip_prefix("has-") {
        let inner = match bracketed(inner) {
            Some(selector) if !selector.contains('&') => {
                format!(":is({})", decode_underscores(selector))
            }
            Some(_) => return None,
            // Hovering a descendant is not guarded like hovering the element
            None if inner == "hover" || inner.starts_with("has-") => return None,
            None => state_selector(inner)?.replacen('&', "", 1),
        };
        return Some(Cow::Owned(format!("&:has({inner})")));
    }
    None
}

/// `group-*`/`peer-*`: the state of an ancestor marked `group`, or of an earlier
/// sibling marked `peer`, the marker named in `group-hover/name`
fn relation_variant(state: &str, marker: &str, relation: &str) -> Option<Variant> {
    let (state, name) = match state.rsplit_once('/') {
        Some((state, name)) if is_name(name) => (state, Some(name)),
        Some(_) => return None,
        None => (state, None),
    };
    let selector = state_selector(state)?;
    let mut element = format!(":where(.{marker}");
    if let Some(name) = name {
        element.push_str("\\/");
        element.push_str(name);
    }
    element.push(')');
    Some(Variant::state(
        Cow::Owned(format!(
            "&:is({}{relation})",
            selector.replacen('&', &element, 1)
        )),
        state == "hover",
    ))
}

/// `[name=value]` of an `aria-[…]`/`data-[…]` variant, the value quoted like
/// Tailwind quotes it
fn attribute_selector(prefix: &str, inner: &str) -> Option<Cow<'static, str>> {
    let inner = decode_underscores(inner);
    let selector = match inner.split_once('=') {
        Some((name, value)) if is_name(name) && !value.is_empty() => {
            if is_quoted(value) {
                format!("&[{prefix}{name}={value}]")
            } else if value.contains(['"', '\'', '\\', ']']) {
                return None;
            } else {
                format!("&[{prefix}{name}=\"{value}\"]")
            }
        }
        None if is_name(&inner) => format!("&[{prefix}{inner}]"),
        _ => return None,
    };
    Some(Cow::Owned(selector))
}

/// `nth-3`, `nth-last-3`, `nth-of-type-3`, `nth-last-of-type-3` and `nth-[…]`
fn nth_selector(position: &str) -> Option<Cow<'static, str>> {
    let (pseudo, position) = if let Some(position) = position.strip_prefix("last-of-type-") {
        ("nth-last-of-type", position)
    } else if let Some(position) = position.strip_prefix("of-type-") {
        ("nth-of-type", position)
    } else if let Some(position) = position.strip_prefix("last-") {
        ("nth-last-child", position)
    } else {
        ("nth-child", position)
    };
    let position = match bracketed(position) {
        Some(inner) => decode_underscores(inner),
        None if position.parse::<u32>().is_ok_and(|n| n > 0) => position.to_string(),
        None => return None,
    };
    Some(Cow::Owned(format!("&:{pseudo}({position})")))
}

/// The condition of `supports-[…]`: a declaration such as `display:grid`, or a
/// parenthesized condition
fn supports_query(inner: &str) -> Option<String> {
    let condition = decode_underscores(inner);
    if condition.starts_with('(') {
        Some(condition)
    } else if condition.contains(':') {
        Some(format!("({condition})"))
    } else {
        None
    }
}

/// A `[…]` variant: selectors with `&` for the element, or an at-rule
fn arbitrary_variant(inner: &str) -> Option<Variant> {
    let text = decode_underscores(inner);
    if text.starts_with('@') {
        let (kind, query) = split_at_rule_key(&text)?;
        return Some(Variant::at_rule(kind, query.to_string()));
    }
    let selectors: Vec<Cow<'static, str>> = split_top_level(&text, ',')
        .into_iter()
        .map(|selector| Cow::Owned(selector.trim().to_string()))
        .collect();
    selectors
        .iter()
        .all(|selector| selector.contains('&'))
        .then_some(Variant {
            selectors,
            ..Variant::default()
        })
}

/// Spacing scale (Tailwind default: 1 unit = 0.25rem = 4px)
static DEFAULT_SPACING: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0px",
    "px" => "1px",
    "0.5" => "0.125rem",
    "1" => "0.25rem",
    "1.5" => "0.375rem",
    "2" => "0.5rem",
    "2.5" => "0.625rem",
    "3" => "0.75rem",
    "3.5" => "0.875rem",
    "4" => "1rem",
    "5" => "1.25rem",
    "6" => "1.5rem",
    "7" => "1.75rem",
    "8" => "2rem",
    "9" => "2.25rem",
    "10" => "2.5rem",
    "11" => "2.75rem",
    "12" => "3rem",
    "14" => "3.5rem",
    "16" => "4rem",
    "20" => "5rem",
    "24" => "6rem",
    "28" => "7rem",
    "32" => "8rem",
    "36" => "9rem",
    "40" => "10rem",
    "44" => "11rem",
    "48" => "12rem",
    "52" => "13rem",
    "56" => "14rem",
    "60" => "15rem",
    "64" => "16rem",
    "72" => "18rem",
    "80" => "20rem",
    "96" => "24rem",
    "auto" => "auto",
    "full" => "100%",
    "1/2" => "50%",
    "1/3" => "33.333333%",
    "2/3" => "66.666667%",
    "1/4" => "25%",
    "2/4" => "50%",
    "3/4" => "75%",
    "1/5" => "20%",
    "2/5" => "40%",
    "3/5" => "60%",
    "4/5" => "80%",
    "1/6" => "16.666667%",
    "2/6" => "33.333333%",
    "3/6" => "50%",
    "4/6" => "66.666667%",
    "5/6" => "83.333333%",
    "1/12" => "8.333333%",
    "2/12" => "16.666667%",
    "3/12" => "25%",
    "4/12" => "33.333333%",
    "5/12" => "41.666667%",
    "6/12" => "50%",
    "7/12" => "58.333333%",
    "8/12" => "66.666667%",
    "9/12" => "75%",
    "10/12" => "83.333333%",
    "11/12" => "91.666667%",
    "screen" => "100vw",
    "svw" => "100svw",
    "lvw" => "100lvw",
    "dvw" => "100dvw",
    "min" => "min-content",
    "max" => "max-content",
    "fit" => "fit-content",
};

/// Font size scale: size and the line height relative to it
static FONT_SIZE_SCALE: phf::Map<&'static str, (&'static str, &'static str)> = phf_map! {
    "xs" => ("0.75rem", "calc(1 / 0.75)"),
    "sm" => ("0.875rem", "calc(1.25 / 0.875)"),
    "base" => ("1rem", "calc(1.5 / 1)"),
    "lg" => ("1.125rem", "calc(1.75 / 1.125)"),
    "xl" => ("1.25rem", "calc(1.75 / 1.25)"),
    "2xl" => ("1.5rem", "calc(2 / 1.5)"),
    "3xl" => ("1.875rem", "calc(2.25 / 1.875)"),
    "4xl" => ("2.25rem", "calc(2.5 / 2.25)"),
    "5xl" => ("3rem", "1"),
    "6xl" => ("3.75rem", "1"),
    "7xl" => ("4.5rem", "1"),
    "8xl" => ("6rem", "1"),
    "9xl" => ("8rem", "1"),
};

/// Font weight scale
static FONT_WEIGHT_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "thin" => "100",
    "extralight" => "200",
    "light" => "300",
    "normal" => "400",
    "medium" => "500",
    "semibold" => "600",
    "bold" => "700",
    "extrabold" => "800",
    "black" => "900",
};

/// The spacing of a step: the project's @theme spacing, else Tailwind's
pub(crate) fn spacing_scale(key: &str) -> Option<Cow<'static, str>> {
    if let Some(value) = theme_value("spacing", key) {
        return Some(Cow::Owned(value));
    }
    if let Some(scaled) = theme_variable("spacing").and_then(|base| scaled(key, &base)) {
        return Some(Cow::Owned(scaled));
    }
    DEFAULT_SPACING
        .get(key)
        .map(|&value| Cow::Borrowed(value))
        .or_else(|| scaled(key, "0.25rem").map(Cow::Owned))
}

/// Border radius scale
static BORDER_RADIUS_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "xs" => "0.125rem",
    "sm" => "0.25rem",
    "" => "0.25rem",
    "md" => "0.375rem",
    "lg" => "0.5rem",
    "xl" => "0.75rem",
    "2xl" => "1rem",
    "3xl" => "1.5rem",
    "4xl" => "2rem",
};

/// Opacity scale
static OPACITY_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0",
    "5" => "0.05",
    "10" => "0.1",
    "15" => "0.15",
    "20" => "0.2",
    "25" => "0.25",
    "30" => "0.3",
    "35" => "0.35",
    "40" => "0.4",
    "45" => "0.45",
    "50" => "0.5",
    "55" => "0.55",
    "60" => "0.6",
    "65" => "0.65",
    "70" => "0.7",
    "75" => "0.75",
    "80" => "0.8",
    "85" => "0.85",
    "90" => "0.9",
    "95" => "0.95",
    "100" => "1",
};

/// Z-index scale
static Z_INDEX_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "0" => "0",
    "10" => "10",
    "20" => "20",
    "30" => "30",
    "40" => "40",
    "50" => "50",
    "auto" => "auto",
};

/// The declarations `class` compiles to, `None` when it stays as written
#[cfg(test)]
pub(crate) fn declarations_of(class: &str) -> Option<Vec<(String, String)>> {
    parse_class(class).map(|class| {
        class
            .declarations
            .into_iter()
            .map(|(property, value)| (property.into_owned(), value.into_owned()))
            .collect()
    })
}

/// What a compiled class sets: each property under each condition, and
/// whether it is important
fn class_keys(class: &TailwindClass) -> BTreeSet<String> {
    let groups = std::iter::once((&class.conditions, &class.declarations)).chain(
        class
            .nested
            .iter()
            .map(|(conditions, declarations)| (conditions, declarations)),
    );
    let mut keys = BTreeSet::new();
    for (conditions, declarations) in groups {
        for condition in conditions {
            for (property, value) in declarations {
                keys.insert(format!(
                    "{condition:?}|{}|{property}|{}",
                    class.level,
                    value.ends_with("!important")
                ));
            }
        }
    }
    keys
}

/// `classes` without the Tailwind classes a later one sets all of again, which
/// is what tailwind-merge does: the later class wins
#[must_use]
pub fn merge_classes(classes: &str) -> String {
    let tokens: Vec<&str> = classes.split_ascii_whitespace().collect();
    let keys: Vec<Option<BTreeSet<String>>> = tokens
        .iter()
        .map(|token| parse_class(token).map(|class| class_keys(&class)))
        .collect();
    tokens
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            keys[*index].as_ref().is_none_or(|earlier| {
                !keys[index + 1..]
                    .iter()
                    .flatten()
                    .any(|later| earlier.is_subset(later))
            })
        })
        .map(|(_, token)| *token)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The class compiled, or `None` when it is not a Tailwind class understood in
/// full, which must then stay in the className as written
#[must_use]
pub fn parse_class(class: &str) -> Option<TailwindClass> {
    let mut parts = split_top_level(class, ':');
    let (name, important) = important(parts.pop()?);
    let Utility {
        mut declarations,
        selector: utility_selector,
        mut nested,
    } = utility(name)?;
    if declarations
        .iter()
        .chain(nested.iter().flat_map(|nested| nested.declarations.iter()))
        .any(|(_, value)| value.contains("theme("))
    {
        return None;
    }
    if important {
        for (_, value) in declarations.iter_mut().chain(
            nested
                .iter_mut()
                .flat_map(|nested| nested.declarations.iter_mut()),
        ) {
            *value = Cow::Owned(format!("{value} !important"));
        }
    }
    let mut level = 0;
    let mut selectors = vec![String::from("&")];
    let mut at_rules = Vec::new();
    let mut content = false;
    // Variants nest left to right, the first one outermost
    for name in parts {
        let variant = variant(name)?;
        level = level.max(variant.level);
        if !variant.selectors.is_empty() {
            selectors = selectors
                .iter()
                .flat_map(|outer| {
                    variant
                        .selectors
                        .iter()
                        .map(move |selector| selector.replace('&', outer))
                })
                .collect();
        }
        at_rules.extend(variant.at_rule);
        content |= variant.content;
    }
    if utility_selector != "&" {
        selectors = selectors
            .iter()
            .map(|selector| utility_selector.replace('&', selector))
            .collect();
    }
    if content
        && !declarations
            .iter()
            .any(|(property, _)| property == "content")
    {
        declarations.insert(
            0,
            (Cow::Borrowed("content"), Cow::Borrowed("var(--tw-content)")),
        );
    }
    let conditions_under = |extra: Option<(AtRuleKind, &str)>| {
        selectors
            .iter()
            .map(|selector| {
                let mut condition =
                    (selector != "&").then(|| StyleSelector::Selector(selector.clone()));
                let nested = extra.map(|(kind, query)| (kind, Cow::Borrowed(query)));
                for (kind, query) in at_rules
                    .iter()
                    .map(|(kind, query)| (*kind, query.clone()))
                    .chain(nested)
                {
                    // `None` when the media queries can never all match
                    condition = Some(StyleSelector::nest_at_rule(
                        condition.as_ref(),
                        kind,
                        &query,
                    )?);
                }
                Some(condition)
            })
            .collect::<Option<Vec<_>>>()
    };
    let conditions = conditions_under(None)?;
    let nested = nested
        .into_iter()
        .map(|nested| {
            Some((
                conditions_under(Some((nested.kind, nested.query)))?,
                nested.declarations,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(TailwindClass {
        level,
        conditions,
        declarations,
        nested,
    })
}

/// Properties a leading `-` negates
static NEGATABLE_PROPERTIES: phf::Set<&'static str> = phf_set! {
    "margin",
    "margin-inline",
    "margin-block",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "margin-inline-start",
    "margin-inline-end",
    "inset",
    "inset-inline",
    "inset-block",
    "top",
    "right",
    "bottom",
    "left",
    "z-index",
    "order",
    "letter-spacing",
    "scroll-margin",
};

/// `name` without the `!` that makes its declarations important, trailing as
/// v4 writes it or leading as v3 did
fn important(name: &str) -> (&str, bool) {
    name.strip_suffix('!')
        .or_else(|| name.strip_prefix('!'))
        .map_or((name, false), |name| (name, true))
}

/// What a utility declares, and on which element: itself, or the children of
/// the element it is on
struct Utility {
    declarations: Vec<Declaration>,
    /// Where the declarations apply, `&` being the element
    selector: &'static str,
    /// Declarations in an at-rule inside the rule
    nested: Vec<Nested>,
}

/// Declarations that go in an at-rule of their own inside the rule of a utility
pub(crate) struct Nested {
    pub kind: AtRuleKind,
    pub query: &'static str,
    pub declarations: Vec<Declaration>,
}

/// The declarations of the utility `token` (`px-4`, `py-2!`) names when it
/// has no variant and applies to the element itself, which `@apply` writes out
pub(crate) fn apply_utility(token: &str) -> Option<Vec<Declaration>> {
    if split_top_level(token, ':').len() > 1 {
        return None;
    }
    let (name, important) = important(token);
    let Utility {
        mut declarations,
        selector,
        nested,
    } = utility(name)?;
    if selector != "&" || !nested.is_empty() {
        return None;
    }
    if important {
        for (_, value) in &mut declarations {
            *value = Cow::Owned(format!("{value} !important"));
        }
    }
    Some(declarations)
}

/// The declarations of a utility and the selector they apply under
fn utility(name: &str) -> Option<Utility> {
    if let Some(declarations) = children_utility(name) {
        return Some(Utility {
            declarations,
            selector: CHILDREN,
            nested: Vec::new(),
        });
    }
    if let Some((declarations, nested)) = nested_utility(name) {
        return Some(Utility {
            declarations,
            selector: "&",
            nested,
        });
    }
    Some(Utility {
        declarations: element_utility(name)?,
        selector: "&",
        nested: Vec::new(),
    })
}

/// The declarations of a utility on the element itself, which a leading `-`
/// negates
fn element_utility(name: &str) -> Option<Vec<Declaration>> {
    let (negative, name) = name
        .strip_prefix('-')
        .map_or((false, name), |name| (true, name));
    if name.starts_with('[') {
        return if negative {
            None
        } else {
            arbitrary_property(name)
        };
    }
    if !negative {
        if let Some(declarations) = custom_declarations(name) {
            return Some(declarations);
        }
        if let Some(declarations) = functional_declarations(name) {
            return Some(declarations);
        }
    }
    if reads_unsupported_theme(name) {
        return None;
    }
    if let Some(declarations) = effect_utility(name, negative) {
        return Some(declarations);
    }
    if let Some(declarations) = misc_utility(name, negative) {
        return Some(declarations);
    }
    if !negative {
        if let Some(declarations) = motion_utility(name) {
            return Some(declarations);
        }
    }
    if let Some((root, value, variable)) = split_arbitrary(name) {
        return arbitrary_utility(root, value, variable, negative);
    }
    if let Some(declarations) = compound_utility(name, negative) {
        return Some(declarations);
    }
    let (property, value) = parse_utility(name)?;
    let value = if negative {
        negate(property, &value)?
    } else {
        value
    };
    Some(vec![(Cow::Borrowed(property), value)])
}

/// `value` negated, for the properties Tailwind lets a `-` negate
fn negate(property: &str, value: &str) -> Option<Cow<'static, str>> {
    (NEGATABLE_PROPERTIES.contains(property)
        && value.starts_with(|c: char| c.is_ascii_digit() || c == '.'))
    .then(|| Cow::Owned(format!("-{value}")))
}

/// `[property:value]`
fn arbitrary_property(name: &str) -> Option<Vec<Declaration>> {
    let (property, value) = bracketed(name)?.split_once(':')?;
    if !is_property_name(property) {
        return None;
    }
    let value = decode_arbitrary_value(value);
    (!value.trim().is_empty()).then(|| vec![(Cow::Owned(property.to_string()), Cow::Owned(value))])
}

/// `root-[value]`, or `root-(--variable)` for `var(--variable)`
fn split_arbitrary(name: &str) -> Option<(&str, &str, bool)> {
    let (open, variable) = match name.as_bytes().last()? {
        b']' => ('[', false),
        b')' => ('(', true),
        _ => return None,
    };
    let start = name.find(open)?;
    let root = name[..start].strip_suffix('-')?;
    let value = &name[start + 1..name.len() - 1];
    (!root.is_empty() && !value.is_empty() && is_balanced(value)).then_some((root, value, variable))
}

/// Roots a leading `-` negates with an arbitrary value
static NEGATABLE_ROOTS: phf::Set<&'static str> = phf_set! {
    "m", "mx", "my", "mt", "mr", "mb", "ml", "ms", "me",
    "inset", "inset-x", "inset-y", "start", "end", "top", "right", "bottom", "left",
    "z", "order", "tracking", "indent",
    "scroll-m", "scroll-mx", "scroll-my", "scroll-mt", "scroll-mr", "scroll-mb", "scroll-ml",
    "scroll-ms", "scroll-me",
    "translate", "translate-x", "translate-y", "rotate", "scale", "scale-x", "scale-y",
    "skew", "skew-x", "skew-y",
};

/// Data types a `[type:value]`/`(type:--variable)` hint names
static DATA_TYPES: phf::Set<&'static str> = phf_set! {
    "color", "length", "percentage", "number", "integer", "url", "image", "position",
    "bg-size", "line-width", "family-name", "generic-name", "absolute-size", "relative-size",
    "angle", "vector",
};

/// The declarations of `root-[value]`/`root-(--variable)`
fn arbitrary_utility(
    root: &str,
    value: &str,
    variable: bool,
    negative: bool,
) -> Option<Vec<Declaration>> {
    let (hint, value) = match value.split_once(':') {
        Some((hint, value)) if DATA_TYPES.contains(hint) => (Some(hint), value),
        // A type Tailwind does not know
        Some((hint, _))
            if hint
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'-') =>
        {
            return None;
        }
        _ => (None, value),
    };
    let mut value = if variable {
        if !value.starts_with("--") {
            return None;
        }
        format!("var({value})")
    } else {
        decode_arbitrary_value(value)
    };
    if value.trim().is_empty() {
        return None;
    }
    if negative {
        if !NEGATABLE_ROOTS.contains(root) {
            return None;
        }
        value = format!("calc({value} * -1)");
    }
    arbitrary_declarations(root, hint, value)
}

/// Roots whose arbitrary value goes to one property
static ARBITRARY_PROPERTIES: phf::Map<&'static str, &'static str> = phf_map! {
    "w" => "width",
    "h" => "height",
    "min-w" => "min-width",
    "max-w" => "max-width",
    "min-h" => "min-height",
    "max-h" => "max-height",
    "p" => "padding",
    "px" => "padding-inline",
    "py" => "padding-block",
    "pt" => "padding-top",
    "pr" => "padding-right",
    "pb" => "padding-bottom",
    "pl" => "padding-left",
    "ps" => "padding-inline-start",
    "pe" => "padding-inline-end",
    "m" => "margin",
    "mx" => "margin-inline",
    "my" => "margin-block",
    "mt" => "margin-top",
    "mr" => "margin-right",
    "mb" => "margin-bottom",
    "ml" => "margin-left",
    "ms" => "margin-inline-start",
    "me" => "margin-inline-end",
    "inset" => "inset",
    "inset-x" => "inset-inline",
    "inset-y" => "inset-block",
    "start" => "inset-inline-start",
    "end" => "inset-inline-end",
    "top" => "top",
    "right" => "right",
    "bottom" => "bottom",
    "left" => "left",
    "gap" => "gap",
    "gap-x" => "column-gap",
    "gap-y" => "row-gap",
    "z" => "z-index",
    "order" => "order",
    "opacity" => "opacity",
    "aspect" => "aspect-ratio",
    "columns" => "columns",
    "basis" => "flex-basis",
    "flex" => "flex",
    "grid-cols" => "grid-template-columns",
    "grid-rows" => "grid-template-rows",
    "col" => "grid-column",
    "row" => "grid-row",
    "col-start" => "grid-column-start",
    "col-end" => "grid-column-end",
    "row-start" => "grid-row-start",
    "row-end" => "grid-row-end",
    "auto-cols" => "grid-auto-columns",
    "auto-rows" => "grid-auto-rows",
    "tracking" => "letter-spacing",
    "indent" => "text-indent",
    "underline-offset" => "text-underline-offset",
    "duration" => "transition-duration",
    "delay" => "transition-delay",
    "ease" => "transition-timing-function",
    "origin" => "transform-origin",
    "rotate" => "rotate",
    "scale" => "scale",
    "cursor" => "cursor",
    "list" => "list-style-type",
    "will-change" => "will-change",
    "object" => "object-position",
    "scroll-m" => "scroll-margin",
    "scroll-mx" => "scroll-margin-inline",
    "scroll-my" => "scroll-margin-block",
    "scroll-mt" => "scroll-margin-top",
    "scroll-mr" => "scroll-margin-right",
    "scroll-mb" => "scroll-margin-bottom",
    "scroll-ml" => "scroll-margin-left",
    "scroll-ms" => "scroll-margin-inline-start",
    "scroll-me" => "scroll-margin-inline-end",
    "scroll-p" => "scroll-padding",
    "scroll-px" => "scroll-padding-inline",
    "scroll-py" => "scroll-padding-block",
    "scroll-pt" => "scroll-padding-top",
    "scroll-pr" => "scroll-padding-right",
    "scroll-pb" => "scroll-padding-bottom",
    "scroll-pl" => "scroll-padding-left",
    "scroll-ps" => "scroll-padding-inline-start",
    "scroll-pe" => "scroll-padding-inline-end",
    "fill" => "fill",
    "accent" => "accent-color",
    "caret" => "caret-color",
};

/// Corners each `rounded-*` side rounds
static RADIUS_SIDES: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "" => &["border-radius"],
    "t" => &["border-top-left-radius", "border-top-right-radius"],
    "r" => &["border-top-right-radius", "border-bottom-right-radius"],
    "b" => &["border-bottom-right-radius", "border-bottom-left-radius"],
    "l" => &["border-top-left-radius", "border-bottom-left-radius"],
    "s" => &["border-start-start-radius", "border-end-start-radius"],
    "e" => &["border-start-end-radius", "border-end-end-radius"],
    "tl" => &["border-top-left-radius"],
    "tr" => &["border-top-right-radius"],
    "br" => &["border-bottom-right-radius"],
    "bl" => &["border-bottom-left-radius"],
    "ss" => &["border-start-start-radius"],
    "se" => &["border-start-end-radius"],
    "ee" => &["border-end-end-radius"],
    "es" => &["border-end-start-radius"],
};

/// Width and color property of each `border-*` side
static BORDER_SIDES: phf::Map<&'static str, (&'static str, &'static str)> = phf_map! {
    "" => ("border-width", "border-color"),
    "x" => ("border-inline-width", "border-inline-color"),
    "y" => ("border-block-width", "border-block-color"),
    "t" => ("border-top-width", "border-top-color"),
    "r" => ("border-right-width", "border-right-color"),
    "b" => ("border-bottom-width", "border-bottom-color"),
    "l" => ("border-left-width", "border-left-color"),
    "s" => ("border-inline-start-width", "border-inline-start-color"),
    "e" => ("border-inline-end-width", "border-inline-end-color"),
};

/// `translate` reading the translation of each axis
const TRANSLATE: &str = "var(--tw-translate-x) var(--tw-translate-y)";
/// `scale` reading the scale of each axis
const SCALE: &str = "var(--tw-scale-x) var(--tw-scale-y)";
/// `transform` composing the rotations and skews no other property takes
pub(crate) const TRANSFORM: &str = "var(--tw-rotate-x,) var(--tw-rotate-y,) var(--tw-rotate-z,) var(--tw-skew-x,) var(--tw-skew-y,)";

/// What an arbitrary value is, for the roots that take several kinds
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueType {
    Length,
    Number,
    Image,
    Color,
    Position,
    Size,
    Family,
}

/// The type a hint names, or that the value has
pub(crate) fn value_type(hint: Option<&str>, value: &str) -> Option<ValueType> {
    Some(match hint {
        Some("length" | "percentage" | "line-width" | "absolute-size" | "relative-size") => {
            ValueType::Length
        }
        Some("number" | "integer") => ValueType::Number,
        Some("image" | "url") => ValueType::Image,
        Some("color") => ValueType::Color,
        Some("position") => ValueType::Position,
        Some("bg-size") => ValueType::Size,
        Some("family-name" | "generic-name") => ValueType::Family,
        Some(_) => return None,
        None if is_number(value) => ValueType::Number,
        None if is_length(value) => ValueType::Length,
        None if is_image(value) => ValueType::Image,
        // Tailwind reads anything else, such as `var(--x)`, as a color
        None => ValueType::Color,
    })
}

/// The declarations of an arbitrary value under `root`
fn arbitrary_declarations(
    root: &str,
    hint: Option<&str>,
    value: String,
) -> Option<Vec<Declaration>> {
    if let Some(&property) = ARBITRARY_PROPERTIES.get(root) {
        return Some(vec![(Cow::Borrowed(property), Cow::Owned(value))]);
    }
    if let Some(side) = root.strip_prefix("rounded") {
        let side = if side.is_empty() {
            side
        } else {
            side.strip_prefix('-')?
        };
        return Some(each(RADIUS_SIDES.get(side)?, &value));
    }
    match root {
        "size" => return Some(each(&["width", "height"], &value)),
        "col-span" | "row-span" => {
            let property = if root == "col-span" {
                "grid-column"
            } else {
                "grid-row"
            };
            return Some(vec![(
                Cow::Borrowed(property),
                Cow::Owned(format!("span {value} / span {value}")),
            )]);
        }
        "content" => {
            return Some(vec![
                (Cow::Borrowed("--tw-content"), Cow::Owned(value)),
                (Cow::Borrowed("content"), Cow::Borrowed("var(--tw-content)")),
            ]);
        }
        "leading" => return Some(each(&["--tw-leading", "line-height"], &value)),
        "translate" | "translate-x" | "translate-y" => {
            return Some(axes(root, "translate", &value, TRANSLATE));
        }
        "scale-x" | "scale-y" => return Some(axes(root, "scale", &value, SCALE)),
        "skew" | "skew-x" | "skew-y" => return Some(skew(root, &value)),
        _ => {}
    }
    let value_type = value_type(hint, &value)?;
    let property = if let Some(side) = root.strip_prefix("border") {
        let side = if side.is_empty() {
            side
        } else {
            side.strip_prefix('-')?
        };
        let &(width, color) = BORDER_SIDES.get(side)?;
        let line_widths = hint.is_none()
            && value
                .split_ascii_whitespace()
                .all(|part| is_length(part) || matches!(part, "thin" | "medium" | "thick"));
        if line_widths || value_type == ValueType::Length {
            width
        } else if value_type == ValueType::Color {
            color
        } else {
            return None;
        }
    } else {
        match (root, value_type) {
            ("text", ValueType::Length) => "font-size",
            ("text", ValueType::Color)
                if hint.is_none() && FONT_SIZE_KEYWORDS.contains(value.as_str()) =>
            {
                "font-size"
            }
            ("text", ValueType::Color) => "color",
            ("bg", ValueType::Image) => "background-image",
            ("bg", ValueType::Color) => "background-color",
            ("bg", ValueType::Position) => "background-position",
            // `bg-[length:…]` sizes the background; a bare length is ambiguous
            ("bg", ValueType::Size | ValueType::Length) if hint.is_some() => "background-size",
            ("outline", ValueType::Length) => "outline-width",
            ("outline", ValueType::Color) => "outline-color",
            ("stroke", ValueType::Length | ValueType::Number) => "stroke-width",
            ("stroke", ValueType::Color) => "stroke",
            ("decoration", ValueType::Length) => "text-decoration-thickness",
            ("decoration", ValueType::Color) => "text-decoration-color",
            ("font", ValueType::Number) => "font-weight",
            ("font", ValueType::Color | ValueType::Family) => "font-family",
            _ => return None,
        }
    };
    Some(vec![(Cow::Borrowed(property), Cow::Owned(value))])
}

/// `value` for every property in `properties`
fn each(properties: &[&'static str], value: &str) -> Vec<Declaration> {
    properties
        .iter()
        .map(|&property| (Cow::Borrowed(property), Cow::Owned(value.to_string())))
        .collect()
}

/// `value` on the axes `root` names, each in its `--tw-<name>-<axis>`
/// variable, which `property` reads as `composite`
fn axes(root: &str, name: &'static str, value: &str, composite: &'static str) -> Vec<Declaration> {
    let axes: &[&str] = match root.strip_prefix(name) {
        Some("-x") => &["x"],
        Some("-y") => &["y"],
        _ => &["x", "y"],
    };
    let mut declarations: Vec<Declaration> = axes
        .iter()
        .map(|axis| {
            (
                Cow::Owned(format!("--tw-{name}-{axis}")),
                Cow::Owned(value.to_string()),
            )
        })
        .collect();
    declarations.push((Cow::Borrowed(name), Cow::Borrowed(composite)));
    declarations
}

/// The skew `root` names by `angle`, composed into `transform`
fn skew(root: &str, angle: &str) -> Vec<Declaration> {
    let axes: &[(&str, &str)] = match root {
        "skew-x" => &[("x", "X")],
        "skew-y" => &[("y", "Y")],
        _ => &[("x", "X"), ("y", "Y")],
    };
    let mut declarations: Vec<Declaration> = axes
        .iter()
        .map(|(axis, function)| {
            (
                Cow::Owned(format!("--tw-skew-{axis}")),
                Cow::Owned(format!("skew{function}({angle})")),
            )
        })
        .collect();
    declarations.push((Cow::Borrowed("transform"), Cow::Borrowed(TRANSFORM)));
    declarations
}

/// Line height of each `leading-*` name
static LEADING_SCALE: phf::Map<&'static str, &'static str> = phf_map! {
    "none" => "1",
    "tight" => "1.25",
    "snug" => "1.375",
    "normal" => "1.5",
    "relaxed" => "1.625",
    "loose" => "2",
};

/// Font-size keywords `text-[…]` takes for a size
static FONT_SIZE_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "xxx-large",
    "larger", "smaller",
};

/// Utilities Tailwind writes with several declarations, and those a leading
/// `-` negates in a way of their own
fn compound_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    if let Some(declarations) = transform_utility(name, negative) {
        return Some(declarations);
    }
    if negative {
        return None;
    }
    let fixed: &[(&'static str, &'static str)] = match name {
        "truncate" => &[
            ("overflow", "hidden"),
            ("text-overflow", "ellipsis"),
            ("white-space", "nowrap"),
        ],
        "sr-only" => &[
            ("position", "absolute"),
            ("width", "1px"),
            ("height", "1px"),
            ("padding", "0"),
            ("margin", "-1px"),
            ("overflow", "hidden"),
            ("clip-path", "inset(50%)"),
            ("white-space", "nowrap"),
            ("border-width", "0"),
        ],
        "not-sr-only" => &[
            ("position", "static"),
            ("width", "auto"),
            ("height", "auto"),
            ("padding", "0"),
            ("margin", "0"),
            ("overflow", "visible"),
            ("clip-path", "none"),
            ("white-space", "normal"),
        ],
        _ => &[],
    };
    if !fixed.is_empty() {
        return Some(
            fixed
                .iter()
                .map(|&(property, value)| (Cow::Borrowed(property), Cow::Borrowed(value)))
                .collect(),
        );
    }
    if let Some(size) = name.strip_prefix("size-") {
        return Some(each(&["width", "height"], &sizing_value(size)?));
    }
    if let Some(declarations) = radius_utility(name) {
        return Some(declarations);
    }
    if let Some(size) = name.strip_prefix("text-") {
        if let Some(declarations) = font_size_utility(size) {
            return Some(declarations);
        }
    }
    if let Some(leading) = name.strip_prefix("leading-") {
        let value = line_height_value(leading)?;
        return Some(vec![
            (Cow::Borrowed("--tw-leading"), value.clone()),
            (Cow::Borrowed("line-height"), value),
        ]);
    }
    None
}

/// A spacing value `size-*` and `translate-*` take
fn sizing_value(size: &str) -> Option<Cow<'static, str>> {
    if matches!(size, "screen" | "svw" | "lvw" | "dvw") {
        return None;
    }
    spacing_scale(size)
}

/// `rounded`, `rounded-lg`, `rounded-t`, `rounded-t-lg`, …
fn radius_utility(name: &str) -> Option<Vec<Declaration>> {
    let rest = name.strip_prefix("rounded")?;
    let (side, size) = if rest.is_empty() {
        ("", "")
    } else {
        let rest = rest.strip_prefix('-')?;
        match rest.split_once('-') {
            Some((side, size)) if RADIUS_SIDES.contains_key(side) => (side, size),
            _ if RADIUS_SIDES.contains_key(rest) => (rest, ""),
            _ => ("", rest),
        }
    };
    let value = match size {
        "none" => Cow::Borrowed("0"),
        "full" => Cow::Borrowed("calc(infinity * 1px)"),
        _ => themed("radius", size, |size| {
            BORDER_RADIUS_SCALE.get(size).copied()
        })?,
    };
    Some(each(RADIUS_SIDES.get(side)?, &value))
}

/// `text-sm`, with the line height of the size unless `text-sm/6` sets one
fn font_size_utility(name: &str) -> Option<Vec<Declaration>> {
    let (size, line_height) = match name.split_once('/') {
        Some((size, line_height)) => (size, Some(line_height_value(line_height)?)),
        None => (name, None),
    };
    let &(font_size, default_line_height) = FONT_SIZE_SCALE.get(size)?;
    // `leading-*` sets `--tw-leading`, which wins over the size's line height
    let line_height = line_height
        .unwrap_or_else(|| Cow::Owned(format!("var(--tw-leading, {default_line_height})")));
    Some(vec![
        (Cow::Borrowed("font-size"), Cow::Borrowed(font_size)),
        (Cow::Borrowed("line-height"), line_height),
    ])
}

/// The line height of `leading-*` and of the `/…` after a font size
fn line_height_value(name: &str) -> Option<Cow<'static, str>> {
    if let Some(&value) = LEADING_SCALE.get(name) {
        return Some(Cow::Borrowed(value));
    }
    if let Some(inner) = bracketed(name) {
        let value = decode_arbitrary_value(inner);
        return (!value.trim().is_empty()).then_some(Cow::Owned(value));
    }
    if let Some(variable) = name
        .strip_prefix('(')
        .and_then(|name| name.strip_suffix(')'))
    {
        return variable
            .starts_with("--")
            .then(|| Cow::Owned(format!("var({variable})")));
    }
    if is_number(name) {
        return spacing_scale(name);
    }
    None
}

/// Translate, rotate, scale and skew, which Tailwind writes to properties of
/// their own so that they compose
fn transform_utility(name: &str, negative: bool) -> Option<Vec<Declaration>> {
    let sign = if negative { "-" } else { "" };
    if let Some(rest) = name.strip_prefix("translate-") {
        let (root, size) = axis_root("translate", rest);
        if matches!(size, "auto" | "min" | "max" | "fit") {
            return None;
        }
        let value = format!("{sign}{}", sizing_value(size)?);
        return Some(axes(root, "translate", &value, TRANSLATE));
    }
    if let Some(angle) = name.strip_prefix("rotate-") {
        return is_number(angle).then(|| {
            vec![(
                Cow::Borrowed("rotate"),
                Cow::Owned(format!("{sign}{angle}deg")),
            )]
        });
    }
    if let Some(rest) = name.strip_prefix("scale-") {
        let (root, amount) = axis_root("scale", rest);
        if !is_integer(amount) {
            return None;
        }
        let value = format!("{sign}{amount}%");
        if root == "scale" {
            let mut declarations = each(&["--tw-scale-x", "--tw-scale-y", "--tw-scale-z"], &value);
            declarations.push((Cow::Borrowed("scale"), Cow::Borrowed(SCALE)));
            return Some(declarations);
        }
        return Some(axes(root, "scale", &value, SCALE));
    }
    if let Some(rest) = name.strip_prefix("skew-") {
        let (root, angle) = axis_root("skew", rest);
        return is_number(angle).then(|| skew(root, &format!("{sign}{angle}deg")));
    }
    None
}

/// The root `name-x`/`name-y`/`name` that `rest` starts with, and its value
fn axis_root<'a>(name: &str, rest: &'a str) -> (&'static str, &'a str) {
    match name {
        "translate" => {
            if let Some(value) = rest.strip_prefix("x-") {
                ("translate-x", value)
            } else if let Some(value) = rest.strip_prefix("y-") {
                ("translate-y", value)
            } else {
                ("translate", rest)
            }
        }
        "scale" => {
            if let Some(value) = rest.strip_prefix("x-") {
                ("scale-x", value)
            } else if let Some(value) = rest.strip_prefix("y-") {
                ("scale-y", value)
            } else {
                ("scale", rest)
            }
        }
        _ => {
            if let Some(value) = rest.strip_prefix("x-") {
                ("skew-x", value)
            } else if let Some(value) = rest.strip_prefix("y-") {
                ("skew-y", value)
            } else {
                ("skew", rest)
            }
        }
    }
}

/// `_` read as a space and `\_` as `_`, as Tailwind reads arbitrary values
fn decode_underscores(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'_') => {
                chars.next();
                decoded.push('_');
            }
            '_' => decoded.push(' '),
            c => decoded.push(c),
        }
    }
    decoded
}

/// An arbitrary value as CSS, the way Tailwind decodes it: underscores are
/// spaces except in `url()` and in the name `var()` reads, and the operators
/// in math functions get the spaces CSS requires around them
pub(crate) fn decode_arbitrary_value(value: &str) -> String {
    resolve_theme_calls(&decode_syntax(value))
}

/// The value of a theme variable `--name`, as `theme(--name)` reads it
fn theme_function_value(name: &str) -> Option<String> {
    let name = name.strip_prefix("--")?;
    if name == "spacing" {
        return Some(theme_variable("spacing").unwrap_or_else(|| String::from("0.25rem")));
    }
    if let Some(key) = name.strip_prefix("color-") {
        return color_value(key).map(Cow::into_owned);
    }
    if let Some(key) = name.strip_prefix("spacing-") {
        return spacing_scale(key).map(Cow::into_owned);
    }
    theme_variable(name)
}

/// `value` with each `theme(--name)` it calls replaced by the value of the
/// theme variable; a call it cannot resolve is left, and the class with it
/// stays as written
fn resolve_theme_calls(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("theme(") {
        let open = start + "theme(".len();
        let length = closing_paren(&rest[open..]);
        let named = rest[..start]
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        let resolved = (!named && open + length < rest.len())
            .then(|| theme_function_value(rest[open..open + length].trim()))
            .flatten();
        if let Some(resolved) = resolved {
            out.push_str(&rest[..start]);
            out.push_str(&resolved);
            rest = &rest[open + length + 1..];
        } else {
            out.push_str(&rest[..open]);
            rest = &rest[open..];
        }
    }
    out.push_str(rest);
    out
}
fn decode_syntax(value: &str) -> String {
    if !value.contains('(') {
        return decode_underscores(value);
    }
    let mut decoded = String::with_capacity(value.len());
    let mut index = 0;
    while index < value.len() {
        index += decode_arguments(&value[index..], &mut decoded);
        if index < value.len() {
            // A `)` closing nothing
            decoded.push(')');
            index += 1;
        }
    }
    add_whitespace_around_math_operators(&decoded)
}

/// Decodes `value` into `decoded` up to the `)` closing the function it is in,
/// returning the bytes read
fn decode_arguments(value: &str, decoded: &mut String) -> usize {
    let mut index = 0;
    while let Some(c) = value[index..].chars().next() {
        match c {
            ')' => return index,
            '(' => {
                let name = function_name(&value[..index]);
                decoded.push('(');
                index += 1;
                if name == "url" || name.ends_with("_url") {
                    let end = closing_paren(&value[index..]);
                    decoded.push_str(&value[index..index + end]);
                    index += end;
                } else {
                    if matches!(name, "var" | "theme")
                        || name.ends_with("_var")
                        || name.ends_with("_theme")
                    {
                        // The first argument names a custom property
                        let end = value[index..]
                            .find([',', ')'])
                            .unwrap_or(value.len() - index);
                        let argument = &value[index..index + end];
                        if !argument.contains('(') {
                            decoded.push_str(&argument.replace("\\_", "_"));
                            index += end;
                        }
                    }
                    index += decode_arguments(&value[index..], decoded);
                }
                if value[index..].starts_with(')') {
                    decoded.push(')');
                    index += 1;
                }
            }
            '\\' if value[index + 1..].starts_with('_') => {
                decoded.push('_');
                index += 2;
            }
            '_' => {
                decoded.push(' ');
                index += 1;
            }
            c => {
                decoded.push(c);
                index += c.len_utf8();
            }
        }
    }
    index
}

/// The name of the function whose `(` follows `before`
fn function_name(before: &str) -> &str {
    let start = before
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .map_or(0, |index| index + 1);
    &before[start..]
}

/// The index of the `)` closing the parentheses `value` is in, or its length
fn closing_paren(value: &str) -> usize {
    let mut depth = 0usize;
    for (index, byte) in value.bytes().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' if depth == 0 => return index,
            b')' => depth -= 1,
            _ => {}
        }
    }
    value.len()
}

/// Math functions whose operators need spaces around them
const MATH_FUNCTIONS: [&str; 19] = [
    "calc", "min", "max", "clamp", "mod", "rem", "sin", "cos", "tan", "asin", "acos", "atan",
    "atan2", "pow", "sqrt", "hypot", "log", "exp", "round",
];

/// `calc(100%-2rem)` as `calc(100% - 2rem)`, ported from Tailwind
fn add_whitespace_around_math_operators(input: &str) -> String {
    if !MATH_FUNCTIONS
        .iter()
        .any(|function| input.contains(function))
    {
        return input.to_string();
    }
    let chars: Vec<char> = input.chars().collect();
    let mut result = String::with_capacity(input.len() + 8);
    // Whether each open parenthesis is in a math function, innermost last
    let mut formattable: Vec<bool> = Vec::new();
    let mut value_end = None;
    let mut last_value_end = None;
    for (index, &c) in chars.iter().enumerate() {
        // A number, and then its unit
        if c.is_ascii_digit() || (value_end.is_some() && (c == '%' || c.is_ascii_alphabetic())) {
            value_end = Some(index);
        } else {
            last_value_end = value_end;
            value_end = None;
        }
        let in_math = formattable.last().copied().unwrap_or(false);
        match c {
            '(' => {
                result.push(c);
                let start = chars[..index]
                    .iter()
                    .rposition(|c| !(c.is_ascii_digit() || c.is_ascii_lowercase()))
                    .map_or(0, |position| position + 1);
                let name: String = chars[start..index].iter().collect();
                formattable
                    .push(MATH_FUNCTIONS.contains(&name.as_str()) || (in_math && name.is_empty()));
            }
            ')' => {
                result.push(c);
                formattable.pop();
            }
            ',' if in_math => result.push_str(", "),
            ' ' if in_math && result.ends_with(' ') => {}
            '+' | '*' | '/' | '-' if in_math => {
                let mut before = result.trim_end().chars().rev();
                let previous = before.next();
                let before_previous = before.next();
                let next = chars.get(index + 1).copied();
                if matches!(previous, Some('e' | 'E'))
                    && before_previous.is_some_and(|c| c.is_ascii_digit())
                {
                    // A scientific notation exponent, such as `1e-3`
                    result.push(c);
                } else if matches!(previous, Some('+' | '*' | '/' | '-' | '(' | ',')) {
                    // The sign of an operand
                    result.push(c);
                } else if chars[index - 1] == ' ' {
                    result.push(c);
                    result.push(' ');
                } else if previous.is_some_and(|c| c.is_ascii_digit() || c == ')')
                    || next.is_some_and(|c| {
                        c.is_ascii_digit() || matches!(c, '(' | '+' | '*' | '/' | '-')
                    })
                    || last_value_end.is_some_and(|end| end + 1 == index)
                {
                    result.push(' ');
                    result.push(c);
                    result.push(' ');
                } else {
                    result.push(c);
                }
            }
            c => result.push(c),
        }
    }
    result
}

/// `value` split at each top-level `separator`, outside brackets and parentheses
pub(crate) fn split_top_level(value: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in value.char_indices() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            c if c == separator && depth == 0 => {
                parts.push(&value[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

/// What `[…]` holds
pub(crate) fn bracketed(value: &str) -> Option<&str> {
    let inner = value.strip_prefix('[')?.strip_suffix(']')?;
    (!inner.is_empty() && is_balanced(inner)).then_some(inner)
}

/// Whether the brackets and parentheses of `value` all close, in order
fn is_balanced(value: &str) -> bool {
    let mut open = Vec::new();
    for c in value.chars() {
        match c {
            '[' | '(' => open.push(c),
            ']' if open.pop() != Some('[') => return false,
            ')' if open.pop() != Some('(') => return false,
            _ => {}
        }
    }
    open.is_empty()
}

/// A name such as `active` in `data-active` or `item` in `group/item`
fn is_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// A quoted string
fn is_quoted(value: &str) -> bool {
    value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
}

/// A CSS property name, custom or not
fn is_property_name(value: &str) -> bool {
    match value.strip_prefix("--") {
        Some(name) => is_name(name),
        None => {
            value
                .trim_start_matches('-')
                .starts_with(|c: char| c.is_ascii_lowercase())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
        }
    }
}

/// A plain number such as `600` or `1.5`
fn is_number(value: &str) -> bool {
    value.parse::<f64>().is_ok()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
}

/// A plain integer such as `110`
fn is_integer(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// Units a length takes
static LENGTH_UNITS: phf::Set<&'static str> = phf_set! {
    "%", "px", "rem", "em", "ex", "ch", "cap", "ic", "lh", "rlh", "rex", "rch", "rcap",
    "ric", "vw", "vh", "vi", "vb", "vmin", "vmax", "svw", "svh", "svi", "svb", "svmin", "svmax",
    "lvw", "lvh", "lvi", "lvb", "lvmin", "lvmax", "dvw", "dvh", "dvi", "dvb", "dvmin", "dvmax",
    "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax", "cm", "mm", "q", "in", "pt", "pc",
};

/// One or more lengths, such as `2rem`, `0`, `calc(…)` or `1px 2px`
fn is_length(value: &str) -> bool {
    if MATH_FUNCTIONS.iter().any(|function| {
        value
            .strip_prefix(function)
            .is_some_and(|rest| rest.starts_with('('))
    }) {
        return true;
    }
    let mut parts = value.split_ascii_whitespace().peekable();
    parts.peek().is_some()
        && parts.all(|part| {
            let number = part.trim_start_matches(['-', '+']);
            let unit_start = number
                .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                .unwrap_or(number.len());
            part == "0"
                || (unit_start > 0
                    && number[..unit_start].parse::<f64>().is_ok()
                    && LENGTH_UNITS.contains(number[unit_start..].to_ascii_lowercase().as_str()))
        })
}

/// An image such as `url(…)` or a gradient
fn is_image(value: &str) -> bool {
    [
        "url(",
        "image(",
        "image-set(",
        "cross-fade(",
        "element(",
        "linear-gradient(",
        "radial-gradient(",
        "conic-gradient(",
        "repeating-linear-gradient(",
        "repeating-radial-gradient(",
        "repeating-conic-gradient(",
    ]
    .iter()
    .any(|function| value.starts_with(function))
}

type ParsedUtility = (&'static str, Cow<'static, str>);

fn tw(property: &'static str, value: impl Into<Cow<'static, str>>) -> ParsedUtility {
    (property, value.into())
}

/// A declaration
pub(crate) fn decl(
    property: impl Into<Cow<'static, str>>,
    value: impl Into<Cow<'static, str>>,
) -> Declaration {
    (property.into(), value.into())
}

/// Whether alue is a whole number written without a sign or leading zeros,
/// which is what v4 takes for the bare values of most utilities
pub(crate) fn is_positive_integer(value: &str) -> bool {
    value == "0"
        || (!value.starts_with('0')
            && value.len() < 16
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && !value.is_empty())
}

/// [value] as the CSS it stands for, or (--variable) as ar(--variable)
pub(crate) fn arbitrary_or_variable(argument: &str) -> Option<String> {
    if let Some(value) = bracketed(argument) {
        return (!value.is_empty() && is_balanced(value)).then(|| decode_arbitrary_value(value));
    }
    let variable = argument
        .strip_prefix('(')
        .and_then(|argument| argument.strip_suffix(')'))?;
    (variable.starts_with("--") && is_balanced(variable)).then(|| format!("var({variable})"))
}

/// The one declaration of a utility from the tables below
fn parse_utility(class: &str) -> Option<ParsedUtility> {
    // Layout utilities
    if let Some(result) = parse_layout_utility(class) {
        return Some(result);
    }

    // Flexbox & Grid
    if let Some(result) = parse_flex_grid_utility(class) {
        return Some(result);
    }

    // Spacing (padding, margin)
    if let Some(result) = parse_spacing_utility(class) {
        return Some(result);
    }

    // Sizing (width, height)
    if let Some(result) = parse_sizing_utility(class) {
        return Some(result);
    }

    // Typography
    if let Some(result) = parse_typography_utility(class) {
        return Some(result);
    }

    // Backgrounds
    if let Some(result) = parse_background_utility(class) {
        return Some(result);
    }

    // Borders
    if let Some(result) = parse_border_utility(class) {
        return Some(result);
    }

    // Effects (shadow, opacity)
    if let Some(result) = parse_effects_utility(class) {
        return Some(result);
    }

    // Transform origin
    if let Some(result) = parse_transform_utility(class) {
        return Some(result);
    }

    // Interactivity
    if let Some(result) = parse_interactivity_utility(class) {
        return Some(result);
    }

    // SVG
    parse_svg_utility(class)
}

/// Parse layout utilities (display, position, visibility, etc.)
fn parse_layout_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    match class {
        // Display
        "block" => Some(tw("display", Cow::Borrowed("block"))),
        "inline-block" => Some(tw("display", Cow::Borrowed("inline-block"))),
        "inline" => Some(tw("display", Cow::Borrowed("inline"))),
        "flex" => Some(tw("display", Cow::Borrowed("flex"))),
        "inline-flex" => Some(tw("display", Cow::Borrowed("inline-flex"))),
        "table" => Some(tw("display", Cow::Borrowed("table"))),
        "inline-table" => Some(tw("display", Cow::Borrowed("inline-table"))),
        "table-caption" => Some(tw("display", Cow::Borrowed("table-caption"))),
        "table-cell" => Some(tw("display", Cow::Borrowed("table-cell"))),
        "table-column" => Some(tw("display", Cow::Borrowed("table-column"))),
        "table-column-group" => Some(tw("display", Cow::Borrowed("table-column-group"))),
        "table-footer-group" => Some(tw("display", Cow::Borrowed("table-footer-group"))),
        "table-header-group" => Some(tw("display", Cow::Borrowed("table-header-group"))),
        "table-row-group" => Some(tw("display", Cow::Borrowed("table-row-group"))),
        "table-row" => Some(tw("display", Cow::Borrowed("table-row"))),
        "flow-root" => Some(tw("display", Cow::Borrowed("flow-root"))),
        "grid" => Some(tw("display", Cow::Borrowed("grid"))),
        "inline-grid" => Some(tw("display", Cow::Borrowed("inline-grid"))),
        "contents" => Some(tw("display", Cow::Borrowed("contents"))),
        "list-item" => Some(tw("display", Cow::Borrowed("list-item"))),
        "hidden" => Some(tw("display", Cow::Borrowed("none"))),

        // Position
        "static" => Some(tw("position", Cow::Borrowed("static"))),
        "fixed" => Some(tw("position", Cow::Borrowed("fixed"))),
        "absolute" => Some(tw("position", Cow::Borrowed("absolute"))),
        "relative" => Some(tw("position", Cow::Borrowed("relative"))),
        "sticky" => Some(tw("position", Cow::Borrowed("sticky"))),

        // Visibility
        "visible" => Some(tw("visibility", Cow::Borrowed("visible"))),
        "invisible" => Some(tw("visibility", Cow::Borrowed("hidden"))),
        "collapse" => Some(tw("visibility", Cow::Borrowed("collapse"))),

        // Box sizing
        "box-border" => Some(tw("box-sizing", Cow::Borrowed("border-box"))),
        "box-content" => Some(tw("box-sizing", Cow::Borrowed("content-box"))),

        // Float
        "float-start" => Some(tw("float", Cow::Borrowed("inline-start"))),
        "float-end" => Some(tw("float", Cow::Borrowed("inline-end"))),
        "float-right" => Some(tw("float", Cow::Borrowed("right"))),
        "float-left" => Some(tw("float", Cow::Borrowed("left"))),
        "float-none" => Some(tw("float", Cow::Borrowed("none"))),

        // Clear
        "clear-start" => Some(tw("clear", Cow::Borrowed("inline-start"))),
        "clear-end" => Some(tw("clear", Cow::Borrowed("inline-end"))),
        "clear-left" => Some(tw("clear", Cow::Borrowed("left"))),
        "clear-right" => Some(tw("clear", Cow::Borrowed("right"))),
        "clear-both" => Some(tw("clear", Cow::Borrowed("both"))),
        "clear-none" => Some(tw("clear", Cow::Borrowed("none"))),

        // Isolation
        "isolate" => Some(tw("isolation", Cow::Borrowed("isolate"))),
        "isolation-auto" => Some(tw("isolation", Cow::Borrowed("auto"))),

        // Object fit
        "object-contain" => Some(tw("object-fit", Cow::Borrowed("contain"))),
        "object-cover" => Some(tw("object-fit", Cow::Borrowed("cover"))),
        "object-fill" => Some(tw("object-fit", Cow::Borrowed("fill"))),
        "object-none" => Some(tw("object-fit", Cow::Borrowed("none"))),
        "object-scale-down" => Some(tw("object-fit", Cow::Borrowed("scale-down"))),

        // Object position
        "object-bottom" => Some(tw("object-position", Cow::Borrowed("bottom"))),
        "object-center" => Some(tw("object-position", Cow::Borrowed("center"))),
        "object-left" => Some(tw("object-position", Cow::Borrowed("left"))),
        "object-left-bottom" => Some(tw("object-position", Cow::Borrowed("left bottom"))),
        "object-left-top" => Some(tw("object-position", Cow::Borrowed("left top"))),
        "object-right" => Some(tw("object-position", Cow::Borrowed("right"))),
        "object-right-bottom" => Some(tw("object-position", Cow::Borrowed("right bottom"))),
        "object-right-top" => Some(tw("object-position", Cow::Borrowed("right top"))),
        "object-top" => Some(tw("object-position", Cow::Borrowed("top"))),

        // Overflow
        "overflow-auto" => Some(tw("overflow", Cow::Borrowed("auto"))),
        "overflow-hidden" => Some(tw("overflow", Cow::Borrowed("hidden"))),
        "overflow-clip" => Some(tw("overflow", Cow::Borrowed("clip"))),
        "overflow-visible" => Some(tw("overflow", Cow::Borrowed("visible"))),
        "overflow-scroll" => Some(tw("overflow", Cow::Borrowed("scroll"))),
        "overflow-x-auto" => Some(tw("overflow-x", Cow::Borrowed("auto"))),
        "overflow-y-auto" => Some(tw("overflow-y", Cow::Borrowed("auto"))),
        "overflow-x-hidden" => Some(tw("overflow-x", Cow::Borrowed("hidden"))),
        "overflow-y-hidden" => Some(tw("overflow-y", Cow::Borrowed("hidden"))),
        "overflow-x-clip" => Some(tw("overflow-x", Cow::Borrowed("clip"))),
        "overflow-y-clip" => Some(tw("overflow-y", Cow::Borrowed("clip"))),
        "overflow-x-visible" => Some(tw("overflow-x", Cow::Borrowed("visible"))),
        "overflow-y-visible" => Some(tw("overflow-y", Cow::Borrowed("visible"))),
        "overflow-x-scroll" => Some(tw("overflow-x", Cow::Borrowed("scroll"))),
        "overflow-y-scroll" => Some(tw("overflow-y", Cow::Borrowed("scroll"))),

        // Overscroll
        "overscroll-auto" => Some(tw("overscroll-behavior", Cow::Borrowed("auto"))),
        "overscroll-contain" => Some(tw("overscroll-behavior", Cow::Borrowed("contain"))),
        "overscroll-none" => Some(tw("overscroll-behavior", Cow::Borrowed("none"))),
        "overscroll-x-auto" => Some(tw("overscroll-behavior-x", Cow::Borrowed("auto"))),
        "overscroll-x-contain" => Some(tw("overscroll-behavior-x", Cow::Borrowed("contain"))),
        "overscroll-x-none" => Some(tw("overscroll-behavior-x", Cow::Borrowed("none"))),
        "overscroll-y-auto" => Some(tw("overscroll-behavior-y", Cow::Borrowed("auto"))),
        "overscroll-y-contain" => Some(tw("overscroll-behavior-y", Cow::Borrowed("contain"))),
        "overscroll-y-none" => Some(tw("overscroll-behavior-y", Cow::Borrowed("none"))),

        _ => {
            // Aspect ratio
            if let Some(rest) = class.strip_prefix("aspect-") {
                let value = match rest {
                    "auto" => Cow::Borrowed("auto"),
                    "square" => Cow::Borrowed("1 / 1"),
                    "video" => Cow::Borrowed("16 / 9"),
                    v => {
                        let (width, height) = v.split_once('/')?;
                        if !is_integer(width) || !is_integer(height) {
                            return None;
                        }
                        Cow::Owned(v.to_string())
                    }
                };
                return Some(tw("aspect-ratio", value));
            }

            // Columns
            if let Some(rest) = class.strip_prefix("columns-") {
                let value = match rest {
                    "auto" => Cow::Borrowed("auto"),
                    "3xs" => Cow::Borrowed("16rem"),
                    "2xs" => Cow::Borrowed("18rem"),
                    "xs" => Cow::Borrowed("20rem"),
                    "sm" => Cow::Borrowed("24rem"),
                    "md" => Cow::Borrowed("28rem"),
                    "lg" => Cow::Borrowed("32rem"),
                    "xl" => Cow::Borrowed("36rem"),
                    "2xl" => Cow::Borrowed("42rem"),
                    "3xl" => Cow::Borrowed("48rem"),
                    "4xl" => Cow::Borrowed("56rem"),
                    "5xl" => Cow::Borrowed("64rem"),
                    "6xl" => Cow::Borrowed("72rem"),
                    "7xl" => Cow::Borrowed("80rem"),
                    v if is_integer(v) => Cow::Owned(v.to_string()),
                    _ => return None,
                };
                return Some(tw("columns", value));
            }

            // Break utilities
            if let Some(rest) = class.strip_prefix("break-") {
                return match rest {
                    "after-auto" => Some(tw("break-after", Cow::Borrowed("auto"))),
                    "after-avoid" => Some(tw("break-after", Cow::Borrowed("avoid"))),
                    "after-all" => Some(tw("break-after", Cow::Borrowed("all"))),
                    "after-avoid-page" => Some(tw("break-after", Cow::Borrowed("avoid-page"))),
                    "after-page" => Some(tw("break-after", Cow::Borrowed("page"))),
                    "after-left" => Some(tw("break-after", Cow::Borrowed("left"))),
                    "after-right" => Some(tw("break-after", Cow::Borrowed("right"))),
                    "after-column" => Some(tw("break-after", Cow::Borrowed("column"))),
                    "before-auto" => Some(tw("break-before", Cow::Borrowed("auto"))),
                    "before-avoid" => Some(tw("break-before", Cow::Borrowed("avoid"))),
                    "before-all" => Some(tw("break-before", Cow::Borrowed("all"))),
                    "before-avoid-page" => Some(tw("break-before", Cow::Borrowed("avoid-page"))),
                    "before-page" => Some(tw("break-before", Cow::Borrowed("page"))),
                    "before-left" => Some(tw("break-before", Cow::Borrowed("left"))),
                    "before-right" => Some(tw("break-before", Cow::Borrowed("right"))),
                    "before-column" => Some(tw("break-before", Cow::Borrowed("column"))),
                    "inside-auto" => Some(tw("break-inside", Cow::Borrowed("auto"))),
                    "inside-avoid" => Some(tw("break-inside", Cow::Borrowed("avoid"))),
                    "inside-avoid-page" => Some(tw("break-inside", Cow::Borrowed("avoid-page"))),
                    "inside-avoid-column" => {
                        Some(tw("break-inside", Cow::Borrowed("avoid-column")))
                    }
                    _ => None,
                };
            }

            // Box decoration break
            if class == "box-decoration-clone" {
                return Some(tw("box-decoration-break", Cow::Borrowed("clone")));
            }
            if class == "box-decoration-slice" {
                return Some(tw("box-decoration-break", Cow::Borrowed("slice")));
            }

            // Z-index
            if let Some(rest) = class.strip_prefix("z-") {
                if let Some(&value) = Z_INDEX_SCALE.get(rest) {
                    return Some(tw("z-index", Cow::Borrowed(value)));
                }
            }

            // Top/Right/Bottom/Left/Inset
            if let Some(rest) = class.strip_prefix("top-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("top", value));
                }
            }
            if let Some(rest) = class.strip_prefix("right-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("right", value));
                }
            }
            if let Some(rest) = class.strip_prefix("bottom-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("bottom", value));
                }
            }
            if let Some(rest) = class.strip_prefix("left-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("left", value));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-x-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("inset-inline", value));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-y-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("inset-block", value));
                }
            }
            if let Some(rest) = class.strip_prefix("inset-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("inset", value));
                }
            }

            None
        }
    }
}

/// Parse flexbox and grid utilities
fn parse_flex_grid_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    match class {
        // Flex basis
        "basis-auto" => Some(tw("flex-basis", Cow::Borrowed("auto"))),
        "basis-full" => Some(tw("flex-basis", Cow::Borrowed("100%"))),

        // Flex direction
        "flex-row" => Some(tw("flex-direction", Cow::Borrowed("row"))),
        "flex-row-reverse" => Some(tw("flex-direction", Cow::Borrowed("row-reverse"))),
        "flex-col" => Some(tw("flex-direction", Cow::Borrowed("column"))),
        "flex-col-reverse" => Some(tw("flex-direction", Cow::Borrowed("column-reverse"))),

        // Flex wrap
        "flex-wrap" => Some(tw("flex-wrap", Cow::Borrowed("wrap"))),
        "flex-wrap-reverse" => Some(tw("flex-wrap", Cow::Borrowed("wrap-reverse"))),
        "flex-nowrap" => Some(tw("flex-wrap", Cow::Borrowed("nowrap"))),

        // Flex
        "flex-1" => Some(tw("flex", Cow::Borrowed("1 1 0%"))),
        "flex-auto" => Some(tw("flex", Cow::Borrowed("1 1 auto"))),
        "flex-initial" => Some(tw("flex", Cow::Borrowed("0 1 auto"))),
        "flex-none" => Some(tw("flex", Cow::Borrowed("none"))),

        // Grow/Shrink
        "grow" => Some(tw("flex-grow", Cow::Borrowed("1"))),
        "grow-0" => Some(tw("flex-grow", Cow::Borrowed("0"))),
        "shrink" => Some(tw("flex-shrink", Cow::Borrowed("1"))),
        "shrink-0" => Some(tw("flex-shrink", Cow::Borrowed("0"))),

        // Order
        "order-first" => Some(tw("order", Cow::Borrowed("-9999"))),
        "order-last" => Some(tw("order", Cow::Borrowed("9999"))),
        "order-none" => Some(tw("order", Cow::Borrowed("0"))),

        // Grid template columns
        "grid-cols-none" => Some(tw("grid-template-columns", Cow::Borrowed("none"))),
        "grid-cols-subgrid" => Some(tw("grid-template-columns", Cow::Borrowed("subgrid"))),

        // Grid template rows
        "grid-rows-none" => Some(tw("grid-template-rows", Cow::Borrowed("none"))),
        "grid-rows-subgrid" => Some(tw("grid-template-rows", Cow::Borrowed("subgrid"))),

        // Grid column
        "col-auto" => Some(tw("grid-column", Cow::Borrowed("auto"))),
        "col-span-full" => Some(tw("grid-column", Cow::Borrowed("1 / -1"))),
        "col-start-auto" => Some(tw("grid-column-start", Cow::Borrowed("auto"))),
        "col-end-auto" => Some(tw("grid-column-end", Cow::Borrowed("auto"))),

        // Grid row
        "row-auto" => Some(tw("grid-row", Cow::Borrowed("auto"))),
        "row-span-full" => Some(tw("grid-row", Cow::Borrowed("1 / -1"))),
        "row-start-auto" => Some(tw("grid-row-start", Cow::Borrowed("auto"))),
        "row-end-auto" => Some(tw("grid-row-end", Cow::Borrowed("auto"))),

        // Grid auto flow
        "grid-flow-row" => Some(tw("grid-auto-flow", Cow::Borrowed("row"))),
        "grid-flow-col" => Some(tw("grid-auto-flow", Cow::Borrowed("column"))),
        "grid-flow-dense" => Some(tw("grid-auto-flow", Cow::Borrowed("dense"))),
        "grid-flow-row-dense" => Some(tw("grid-auto-flow", Cow::Borrowed("row dense"))),
        "grid-flow-col-dense" => Some(tw("grid-auto-flow", Cow::Borrowed("column dense"))),

        // Grid auto columns
        "auto-cols-auto" => Some(tw("grid-auto-columns", Cow::Borrowed("auto"))),
        "auto-cols-min" => Some(tw("grid-auto-columns", Cow::Borrowed("min-content"))),
        "auto-cols-max" => Some(tw("grid-auto-columns", Cow::Borrowed("max-content"))),
        "auto-cols-fr" => Some(tw("grid-auto-columns", Cow::Borrowed("minmax(0, 1fr)"))),

        // Grid auto rows
        "auto-rows-auto" => Some(tw("grid-auto-rows", Cow::Borrowed("auto"))),
        "auto-rows-min" => Some(tw("grid-auto-rows", Cow::Borrowed("min-content"))),
        "auto-rows-max" => Some(tw("grid-auto-rows", Cow::Borrowed("max-content"))),
        "auto-rows-fr" => Some(tw("grid-auto-rows", Cow::Borrowed("minmax(0, 1fr)"))),

        // Justify content
        "justify-normal" => Some(tw("justify-content", Cow::Borrowed("normal"))),
        "justify-start" => Some(tw("justify-content", Cow::Borrowed("flex-start"))),
        "justify-end" => Some(tw("justify-content", Cow::Borrowed("flex-end"))),
        "justify-center" => Some(tw("justify-content", Cow::Borrowed("center"))),
        "justify-between" => Some(tw("justify-content", Cow::Borrowed("space-between"))),
        "justify-around" => Some(tw("justify-content", Cow::Borrowed("space-around"))),
        "justify-evenly" => Some(tw("justify-content", Cow::Borrowed("space-evenly"))),
        "justify-stretch" => Some(tw("justify-content", Cow::Borrowed("stretch"))),

        // Justify items
        "justify-items-start" => Some(tw("justify-items", Cow::Borrowed("start"))),
        "justify-items-end" => Some(tw("justify-items", Cow::Borrowed("end"))),
        "justify-items-center" => Some(tw("justify-items", Cow::Borrowed("center"))),
        "justify-items-stretch" => Some(tw("justify-items", Cow::Borrowed("stretch"))),

        // Justify self
        "justify-self-auto" => Some(tw("justify-self", Cow::Borrowed("auto"))),
        "justify-self-start" => Some(tw("justify-self", Cow::Borrowed("start"))),
        "justify-self-end" => Some(tw("justify-self", Cow::Borrowed("end"))),
        "justify-self-center" => Some(tw("justify-self", Cow::Borrowed("center"))),
        "justify-self-stretch" => Some(tw("justify-self", Cow::Borrowed("stretch"))),

        // Align content
        "content-normal" => Some(tw("align-content", Cow::Borrowed("normal"))),
        "content-center" => Some(tw("align-content", Cow::Borrowed("center"))),
        "content-start" => Some(tw("align-content", Cow::Borrowed("flex-start"))),
        "content-end" => Some(tw("align-content", Cow::Borrowed("flex-end"))),
        "content-between" => Some(tw("align-content", Cow::Borrowed("space-between"))),
        "content-around" => Some(tw("align-content", Cow::Borrowed("space-around"))),
        "content-evenly" => Some(tw("align-content", Cow::Borrowed("space-evenly"))),
        "content-baseline" => Some(tw("align-content", Cow::Borrowed("baseline"))),
        "content-stretch" => Some(tw("align-content", Cow::Borrowed("stretch"))),

        // Align items
        "items-start" => Some(tw("align-items", Cow::Borrowed("flex-start"))),
        "items-end" => Some(tw("align-items", Cow::Borrowed("flex-end"))),
        "items-center" => Some(tw("align-items", Cow::Borrowed("center"))),
        "items-baseline" => Some(tw("align-items", Cow::Borrowed("baseline"))),
        "items-stretch" => Some(tw("align-items", Cow::Borrowed("stretch"))),

        // Align self
        "self-auto" => Some(tw("align-self", Cow::Borrowed("auto"))),
        "self-start" => Some(tw("align-self", Cow::Borrowed("flex-start"))),
        "self-end" => Some(tw("align-self", Cow::Borrowed("flex-end"))),
        "self-center" => Some(tw("align-self", Cow::Borrowed("center"))),
        "self-stretch" => Some(tw("align-self", Cow::Borrowed("stretch"))),
        "self-baseline" => Some(tw("align-self", Cow::Borrowed("baseline"))),

        // Place content
        "place-content-center" => Some(tw("place-content", Cow::Borrowed("center"))),
        "place-content-start" => Some(tw("place-content", Cow::Borrowed("start"))),
        "place-content-end" => Some(tw("place-content", Cow::Borrowed("end"))),
        "place-content-between" => Some(tw("place-content", Cow::Borrowed("space-between"))),
        "place-content-around" => Some(tw("place-content", Cow::Borrowed("space-around"))),
        "place-content-evenly" => Some(tw("place-content", Cow::Borrowed("space-evenly"))),
        "place-content-baseline" => Some(tw("place-content", Cow::Borrowed("baseline"))),
        "place-content-stretch" => Some(tw("place-content", Cow::Borrowed("stretch"))),

        // Place items
        "place-items-start" => Some(tw("place-items", Cow::Borrowed("start"))),
        "place-items-end" => Some(tw("place-items", Cow::Borrowed("end"))),
        "place-items-center" => Some(tw("place-items", Cow::Borrowed("center"))),
        "place-items-baseline" => Some(tw("place-items", Cow::Borrowed("baseline"))),
        "place-items-stretch" => Some(tw("place-items", Cow::Borrowed("stretch"))),

        // Place self
        "place-self-auto" => Some(tw("place-self", Cow::Borrowed("auto"))),
        "place-self-start" => Some(tw("place-self", Cow::Borrowed("start"))),
        "place-self-end" => Some(tw("place-self", Cow::Borrowed("end"))),
        "place-self-center" => Some(tw("place-self", Cow::Borrowed("center"))),
        "place-self-stretch" => Some(tw("place-self", Cow::Borrowed("stretch"))),

        _ => {
            // Flex basis with spacing scale
            if let Some(rest) = class.strip_prefix("basis-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("flex-basis", value));
                }
            }

            // Order with number
            if let Some(rest) = class.strip_prefix("order-") {
                if is_integer(rest) {
                    return Some(tw("order", Cow::Owned(rest.to_string())));
                }
            }

            // Grid cols
            if let Some(rest) = class.strip_prefix("grid-cols-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some(tw(
                        "grid-template-columns",
                        format!("repeat({n}, minmax(0, 1fr))"),
                    ));
                }
            }

            // Grid rows
            if let Some(rest) = class.strip_prefix("grid-rows-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some(tw(
                        "grid-template-rows",
                        format!("repeat({n}, minmax(0, 1fr))"),
                    ));
                }
            }

            // Col span
            if let Some(rest) = class.strip_prefix("col-span-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some(tw("grid-column", format!("span {n} / span {n}")));
                }
            }

            // Col start
            if let Some(rest) = class.strip_prefix("col-start-") {
                if is_integer(rest) {
                    return Some(tw("grid-column-start", Cow::Owned(rest.to_string())));
                }
            }

            // Col end
            if let Some(rest) = class.strip_prefix("col-end-") {
                if is_integer(rest) {
                    return Some(tw("grid-column-end", Cow::Owned(rest.to_string())));
                }
            }

            // Row span
            if let Some(rest) = class.strip_prefix("row-span-") {
                if let Ok(n) = rest.parse::<u32>() {
                    return Some(tw("grid-row", format!("span {n} / span {n}")));
                }
            }

            // Row start
            if let Some(rest) = class.strip_prefix("row-start-") {
                if is_integer(rest) {
                    return Some(tw("grid-row-start", Cow::Owned(rest.to_string())));
                }
            }

            // Row end
            if let Some(rest) = class.strip_prefix("row-end-") {
                if is_integer(rest) {
                    return Some(tw("grid-row-end", Cow::Owned(rest.to_string())));
                }
            }

            // Gap
            if let Some(rest) = class.strip_prefix("gap-x-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("column-gap", value));
                }
            }
            if let Some(rest) = class.strip_prefix("gap-y-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("row-gap", value));
                }
            }
            if let Some(rest) = class.strip_prefix("gap-") {
                if let Some(value) = spacing_scale(rest) {
                    return Some(tw("gap", value));
                }
            }

            None
        }
    }
}

/// Parse spacing utilities (padding, margin, space)
fn parse_spacing_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Padding
    if let Some(rest) = class.strip_prefix("px-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-inline", value));
        }
    }
    if let Some(rest) = class.strip_prefix("py-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-block", value));
        }
    }
    if let Some(rest) = class.strip_prefix("pt-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-top", value));
        }
    }
    if let Some(rest) = class.strip_prefix("pr-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-right", value));
        }
    }
    if let Some(rest) = class.strip_prefix("pb-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-bottom", value));
        }
    }
    if let Some(rest) = class.strip_prefix("pl-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-left", value));
        }
    }
    if let Some(rest) = class.strip_prefix("ps-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-inline-start", value));
        }
    }
    if let Some(rest) = class.strip_prefix("pe-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding-inline-end", value));
        }
    }
    if let Some(rest) = class.strip_prefix("p-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("padding", value));
        }
    }

    // Margin
    if let Some(rest) = class.strip_prefix("mx-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-inline", value));
        }
    }
    if let Some(rest) = class.strip_prefix("my-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-block", value));
        }
    }
    if let Some(rest) = class.strip_prefix("mt-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-top", value));
        }
    }
    if let Some(rest) = class.strip_prefix("mr-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-right", value));
        }
    }
    if let Some(rest) = class.strip_prefix("mb-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-bottom", value));
        }
    }
    if let Some(rest) = class.strip_prefix("ml-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-left", value));
        }
    }
    if let Some(rest) = class.strip_prefix("ms-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-inline-start", value));
        }
    }
    if let Some(rest) = class.strip_prefix("me-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin-inline-end", value));
        }
    }
    if let Some(rest) = class.strip_prefix("m-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("margin", value));
        }
    }

    None
}

/// Parse sizing utilities (width, height, min/max)
fn parse_sizing_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Width
    if let Some(rest) = class.strip_prefix("w-") {
        if let Some(value) = spacing_scale(rest) {
            return Some(tw("width", value));
        }
    }

    // Min width
    if let Some(rest) = class.strip_prefix("min-w-") {
        let value = match rest {
            "0" => Cow::Borrowed("0px"),
            "full" => Cow::Borrowed("100%"),
            "min" => Cow::Borrowed("min-content"),
            "max" => Cow::Borrowed("max-content"),
            "fit" => Cow::Borrowed("fit-content"),
            _ => spacing_scale(rest)?,
        };
        return Some(tw("min-width", value));
    }

    // Max width
    if let Some(rest) = class.strip_prefix("max-w-") {
        let value = match rest {
            "none" => Cow::Borrowed("none"),
            "0" => Cow::Borrowed("0rem"),
            "xs" => Cow::Borrowed("20rem"),
            "sm" => Cow::Borrowed("24rem"),
            "md" => Cow::Borrowed("28rem"),
            "lg" => Cow::Borrowed("32rem"),
            "xl" => Cow::Borrowed("36rem"),
            "2xl" => Cow::Borrowed("42rem"),
            "3xl" => Cow::Borrowed("48rem"),
            "4xl" => Cow::Borrowed("56rem"),
            "5xl" => Cow::Borrowed("64rem"),
            "6xl" => Cow::Borrowed("72rem"),
            "7xl" => Cow::Borrowed("80rem"),
            "full" => Cow::Borrowed("100%"),
            "min" => Cow::Borrowed("min-content"),
            "max" => Cow::Borrowed("max-content"),
            "fit" => Cow::Borrowed("fit-content"),
            "prose" => Cow::Borrowed("65ch"),
            "screen-sm" => Cow::Borrowed("640px"),
            "screen-md" => Cow::Borrowed("768px"),
            "screen-lg" => Cow::Borrowed("1024px"),
            "screen-xl" => Cow::Borrowed("1280px"),
            "screen-2xl" => Cow::Borrowed("1536px"),
            _ => spacing_scale(rest)?,
        };
        return Some(tw("max-width", value));
    }

    // Height
    if let Some(rest) = class.strip_prefix("h-") {
        let value = match rest {
            "screen" => Cow::Borrowed("100vh"),
            "svh" => Cow::Borrowed("100svh"),
            "lvh" => Cow::Borrowed("100lvh"),
            "dvh" => Cow::Borrowed("100dvh"),
            _ => spacing_scale(rest)?,
        };
        return Some(tw("height", value));
    }

    // Min height
    if let Some(rest) = class.strip_prefix("min-h-") {
        let value = match rest {
            "0" => Cow::Borrowed("0px"),
            "full" => Cow::Borrowed("100%"),
            "screen" => Cow::Borrowed("100vh"),
            "svh" => Cow::Borrowed("100svh"),
            "lvh" => Cow::Borrowed("100lvh"),
            "dvh" => Cow::Borrowed("100dvh"),
            "min" => Cow::Borrowed("min-content"),
            "max" => Cow::Borrowed("max-content"),
            "fit" => Cow::Borrowed("fit-content"),
            _ => spacing_scale(rest)?,
        };
        return Some(tw("min-height", value));
    }

    // Max height
    if let Some(rest) = class.strip_prefix("max-h-") {
        let value = match rest {
            "none" => Cow::Borrowed("none"),
            "full" => Cow::Borrowed("100%"),
            "screen" => Cow::Borrowed("100vh"),
            "svh" => Cow::Borrowed("100svh"),
            "lvh" => Cow::Borrowed("100lvh"),
            "dvh" => Cow::Borrowed("100dvh"),
            "min" => Cow::Borrowed("min-content"),
            "max" => Cow::Borrowed("max-content"),
            "fit" => Cow::Borrowed("fit-content"),
            _ => spacing_scale(rest)?,
        };
        return Some(tw("max-height", value));
    }

    None
}

/// Parse typography utilities
fn parse_typography_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Font family
    match class {
        "font-sans" => {
            return Some(tw(
                "font-family",
                Cow::Borrowed(
                    "ui-sans-serif, system-ui, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'",
                ),
            ));
        }
        "font-serif" => {
            return Some(tw(
                "font-family",
                Cow::Borrowed("ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif"),
            ));
        }
        "font-mono" => {
            return Some(tw(
                "font-family",
                Cow::Borrowed(
                    "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace",
                ),
            ));
        }
        _ => {}
    }

    // Text color (font sizes are `compound_utility`'s)
    if let Some(rest) = class.strip_prefix("text-") {
        if let Some(color) = color_value(rest) {
            return Some(tw("color", color));
        }
        // Text alignment
        match rest {
            "left" => return Some(tw("text-align", Cow::Borrowed("left"))),
            "center" => return Some(tw("text-align", Cow::Borrowed("center"))),
            "right" => return Some(tw("text-align", Cow::Borrowed("right"))),
            "justify" => return Some(tw("text-align", Cow::Borrowed("justify"))),
            "start" => return Some(tw("text-align", Cow::Borrowed("start"))),
            "end" => return Some(tw("text-align", Cow::Borrowed("end"))),
            _ => {}
        }
    }

    // Font weight
    if let Some(rest) = class.strip_prefix("font-") {
        if let Some(&weight) = FONT_WEIGHT_SCALE.get(rest) {
            return Some(tw("font-weight", Cow::Borrowed(weight)));
        }
    }

    // Font style
    match class {
        "italic" => return Some(tw("font-style", Cow::Borrowed("italic"))),
        "not-italic" => return Some(tw("font-style", Cow::Borrowed("normal"))),
        _ => {}
    }

    // Text decoration
    match class {
        "underline" => return Some(tw("text-decoration-line", Cow::Borrowed("underline"))),
        "overline" => return Some(tw("text-decoration-line", Cow::Borrowed("overline"))),
        "line-through" => {
            return Some(tw("text-decoration-line", Cow::Borrowed("line-through")));
        }
        "no-underline" => return Some(tw("text-decoration-line", Cow::Borrowed("none"))),
        _ => {}
    }

    // Text transform
    match class {
        "uppercase" => return Some(tw("text-transform", Cow::Borrowed("uppercase"))),
        "lowercase" => return Some(tw("text-transform", Cow::Borrowed("lowercase"))),
        "capitalize" => return Some(tw("text-transform", Cow::Borrowed("capitalize"))),
        "normal-case" => return Some(tw("text-transform", Cow::Borrowed("none"))),
        _ => {}
    }

    // Text overflow
    match class {
        "text-ellipsis" => return Some(tw("text-overflow", Cow::Borrowed("ellipsis"))),
        "text-clip" => return Some(tw("text-overflow", Cow::Borrowed("clip"))),
        _ => {}
    }

    // Text wrap
    match class {
        "text-wrap" => return Some(tw("text-wrap", Cow::Borrowed("wrap"))),
        "text-nowrap" => return Some(tw("text-wrap", Cow::Borrowed("nowrap"))),
        "text-balance" => return Some(tw("text-wrap", Cow::Borrowed("balance"))),
        "text-pretty" => return Some(tw("text-wrap", Cow::Borrowed("pretty"))),
        _ => {}
    }

    // Whitespace
    if let Some(rest) = class.strip_prefix("whitespace-") {
        return matches!(
            rest,
            "normal" | "nowrap" | "pre" | "pre-line" | "pre-wrap" | "break-spaces"
        )
        .then(|| tw("white-space", Cow::Owned(rest.to_string())));
    }

    // Word break
    match class {
        "break-normal" => return Some(tw("word-break", Cow::Borrowed("normal"))),
        "break-words" => return Some(tw("overflow-wrap", Cow::Borrowed("break-word"))),
        "break-all" => return Some(tw("word-break", Cow::Borrowed("break-all"))),
        "break-keep" => return Some(tw("word-break", Cow::Borrowed("keep-all"))),
        _ => {}
    }

    // Hyphens
    if let Some(rest) = class.strip_prefix("hyphens-") {
        return matches!(rest, "none" | "manual" | "auto")
            .then(|| tw("hyphens", Cow::Owned(rest.to_string())));
    }

    // Letter spacing
    if let Some(rest) = class.strip_prefix("tracking-") {
        let value = match rest {
            "tighter" => "-0.05em",
            "tight" => "-0.025em",
            "normal" => "0em",
            "wide" => "0.025em",
            "wider" => "0.05em",
            "widest" => "0.1em",
            _ => return None,
        };
        return Some(tw("letter-spacing", Cow::Borrowed(value)));
    }

    // List style type
    if let Some(rest) = class.strip_prefix("list-") {
        match rest {
            "inside" => return Some(tw("list-style-position", Cow::Borrowed("inside"))),
            "outside" => return Some(tw("list-style-position", Cow::Borrowed("outside"))),
            "none" => return Some(tw("list-style-type", Cow::Borrowed("none"))),
            "disc" => return Some(tw("list-style-type", Cow::Borrowed("disc"))),
            "decimal" => return Some(tw("list-style-type", Cow::Borrowed("decimal"))),
            _ => {}
        }
    }

    // Vertical align
    if let Some(rest) = class.strip_prefix("align-") {
        return matches!(
            rest,
            "baseline" | "top" | "middle" | "bottom" | "text-top" | "text-bottom" | "sub" | "super"
        )
        .then(|| tw("vertical-align", Cow::Owned(rest.to_string())));
    }

    // Content
    if let Some(rest) = class.strip_prefix("content-") {
        if rest == "none" {
            return Some(tw("content", Cow::Borrowed("none")));
        }
    }

    None
}

/// Parse background utilities
fn parse_background_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Background color
    if let Some(rest) = class.strip_prefix("bg-") {
        // Check if it's a color
        if let Some(color) = color_value(rest) {
            return Some(tw("background-color", color));
        }
        // Background attachment
        match rest {
            "fixed" => return Some(tw("background-attachment", Cow::Borrowed("fixed"))),
            "local" => return Some(tw("background-attachment", Cow::Borrowed("local"))),
            "scroll" => return Some(tw("background-attachment", Cow::Borrowed("scroll"))),
            // Background clip
            "clip-border" => {
                return Some(tw("background-clip", Cow::Borrowed("border-box")));
            }
            "clip-padding" => {
                return Some(tw("background-clip", Cow::Borrowed("padding-box")));
            }
            "clip-content" => {
                return Some(tw("background-clip", Cow::Borrowed("content-box")));
            }
            "clip-text" => return Some(tw("background-clip", Cow::Borrowed("text"))),
            // Background origin
            "origin-border" => {
                return Some(tw("background-origin", Cow::Borrowed("border-box")));
            }
            "origin-padding" => {
                return Some(tw("background-origin", Cow::Borrowed("padding-box")));
            }
            "origin-content" => {
                return Some(tw("background-origin", Cow::Borrowed("content-box")));
            }
            // Background position
            "bottom" => return Some(tw("background-position", Cow::Borrowed("bottom"))),
            "center" => return Some(tw("background-position", Cow::Borrowed("center"))),
            "left" => return Some(tw("background-position", Cow::Borrowed("left"))),
            "left-bottom" => {
                return Some(tw("background-position", Cow::Borrowed("left bottom")));
            }
            "left-top" => return Some(tw("background-position", Cow::Borrowed("left top"))),
            "right" => return Some(tw("background-position", Cow::Borrowed("right"))),
            "right-bottom" => {
                return Some(tw("background-position", Cow::Borrowed("right bottom")));
            }
            "right-top" => {
                return Some(tw("background-position", Cow::Borrowed("right top")));
            }
            "top" => return Some(tw("background-position", Cow::Borrowed("top"))),
            // Background repeat
            "repeat" => return Some(tw("background-repeat", Cow::Borrowed("repeat"))),
            "no-repeat" => return Some(tw("background-repeat", Cow::Borrowed("no-repeat"))),
            "repeat-x" => return Some(tw("background-repeat", Cow::Borrowed("repeat-x"))),
            "repeat-y" => return Some(tw("background-repeat", Cow::Borrowed("repeat-y"))),
            "repeat-round" => return Some(tw("background-repeat", Cow::Borrowed("round"))),
            "repeat-space" => return Some(tw("background-repeat", Cow::Borrowed("space"))),
            // Background size
            "auto" => return Some(tw("background-size", Cow::Borrowed("auto"))),
            "cover" => return Some(tw("background-size", Cow::Borrowed("cover"))),
            "contain" => return Some(tw("background-size", Cow::Borrowed("contain"))),
            // Gradients
            "none" => return Some(tw("background-image", Cow::Borrowed("none"))),
            _ => {}
        }
    }

    None
}

/// Parse border utilities
fn parse_border_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Border radius is compound_utility's

    // Border width
    if let Some(rest) = class.strip_prefix("border-") {
        // Border color
        if let Some(color) = color_value(rest) {
            return Some(tw("border-color", color));
        }

        // Border collapse (for tables)
        if rest == "collapse" {
            return Some(tw("border-collapse", Cow::Borrowed("collapse")));
        }
        if rest == "separate" {
            return Some(tw("border-collapse", Cow::Borrowed("separate")));
        }
    }

    // Outline color
    if let Some(rest) = class.strip_prefix("outline-") {
        if let Some(color) = color_value(rest) {
            return Some(tw("outline-color", color));
        }
    }

    None
}

/// Parse effects utilities (shadow, opacity, mix-blend, etc.)
fn parse_effects_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Opacity
    if let Some(rest) = class.strip_prefix("opacity-") {
        if let Some(&value) = OPACITY_SCALE.get(rest) {
            return Some(tw("opacity", Cow::Borrowed(value)));
        }
    }

    // Mix blend mode
    if let Some(rest) = class.strip_prefix("mix-blend-") {
        return (BLEND_MODES.contains(rest) || matches!(rest, "plus-darker" | "plus-lighter"))
            .then(|| tw("mix-blend-mode", Cow::Owned(rest.to_string())));
    }

    // Background blend mode
    if let Some(rest) = class.strip_prefix("bg-blend-") {
        return BLEND_MODES
            .contains(rest)
            .then(|| tw("background-blend-mode", Cow::Owned(rest.to_string())));
    }

    None
}

/// Blend modes `mix-blend-*` and `bg-blend-*` take
static BLEND_MODES: phf::Set<&'static str> = phf_set! {
    "normal", "multiply", "screen", "overlay", "darken", "lighten", "color-dodge", "color-burn",
    "hard-light", "soft-light", "difference", "exclusion", "hue", "saturation", "color",
    "luminosity",
};

/// Parse transform origin utilities (translate, rotate, scale and skew are
/// `transform_utility`'s)
fn parse_transform_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    if let Some(rest) = class.strip_prefix("origin-") {
        let value = match rest {
            "center" => Cow::Borrowed("center"),
            "top" => Cow::Borrowed("top"),
            "top-right" => Cow::Borrowed("top right"),
            "right" => Cow::Borrowed("right"),
            "bottom-right" => Cow::Borrowed("bottom right"),
            "bottom" => Cow::Borrowed("bottom"),
            "bottom-left" => Cow::Borrowed("bottom left"),
            "left" => Cow::Borrowed("left"),
            "top-left" => Cow::Borrowed("top left"),
            _ => return None,
        };
        return Some(tw("transform-origin", value));
    }

    None
}

/// Parse interactivity utilities (cursor, pointer-events, resize, etc.)
fn parse_interactivity_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Accent color
    if let Some(rest) = class.strip_prefix("accent-") {
        if rest == "auto" {
            return Some(tw("accent-color", Cow::Borrowed("auto")));
        }
        if let Some(color) = color_value(rest) {
            return Some(tw("accent-color", color));
        }
    }

    // Appearance
    match class {
        "appearance-none" => return Some(tw("appearance", Cow::Borrowed("none"))),
        "appearance-auto" => return Some(tw("appearance", Cow::Borrowed("auto"))),
        _ => {}
    }

    // Cursor
    if let Some(rest) = class.strip_prefix("cursor-") {
        return CURSORS
            .contains(rest)
            .then(|| tw("cursor", Cow::Owned(rest.to_string())));
    }

    // Caret color
    if let Some(rest) = class.strip_prefix("caret-") {
        if let Some(color) = color_value(rest) {
            return Some(tw("caret-color", color));
        }
    }

    // Pointer events
    if let Some(rest) = class.strip_prefix("pointer-events-") {
        return matches!(rest, "none" | "auto")
            .then(|| tw("pointer-events", Cow::Owned(rest.to_string())));
    }

    // Resize
    match class {
        "resize-none" => return Some(tw("resize", Cow::Borrowed("none"))),
        "resize-y" => return Some(tw("resize", Cow::Borrowed("vertical"))),
        "resize-x" => return Some(tw("resize", Cow::Borrowed("horizontal"))),
        "resize" => return Some(tw("resize", Cow::Borrowed("both"))),
        _ => {}
    }

    // Scroll behavior
    if let Some(rest) = class.strip_prefix("scroll-") {
        match rest {
            "auto" => return Some(tw("scroll-behavior", Cow::Borrowed("auto"))),
            "smooth" => return Some(tw("scroll-behavior", Cow::Borrowed("smooth"))),
            _ => {}
        }
    }

    // Scroll snap
    if let Some(rest) = class.strip_prefix("snap-") {
        match rest {
            "start" => return Some(tw("scroll-snap-align", Cow::Borrowed("start"))),
            "end" => return Some(tw("scroll-snap-align", Cow::Borrowed("end"))),
            "center" => return Some(tw("scroll-snap-align", Cow::Borrowed("center"))),
            "align-none" => return Some(tw("scroll-snap-align", Cow::Borrowed("none"))),
            "none" => return Some(tw("scroll-snap-type", Cow::Borrowed("none"))),
            "x" => {
                return Some(tw(
                    "scroll-snap-type",
                    Cow::Borrowed("x var(--tw-scroll-snap-strictness)"),
                ));
            }
            "y" => {
                return Some(tw(
                    "scroll-snap-type",
                    Cow::Borrowed("y var(--tw-scroll-snap-strictness)"),
                ));
            }
            "both" => {
                return Some(tw(
                    "scroll-snap-type",
                    Cow::Borrowed("both var(--tw-scroll-snap-strictness)"),
                ));
            }
            "mandatory" => {
                return Some(tw(
                    "--tw-scroll-snap-strictness",
                    Cow::Borrowed("mandatory"),
                ));
            }
            "proximity" => {
                return Some(tw(
                    "--tw-scroll-snap-strictness",
                    Cow::Borrowed("proximity"),
                ));
            }
            "normal" => return Some(tw("scroll-snap-stop", Cow::Borrowed("normal"))),
            "always" => return Some(tw("scroll-snap-stop", Cow::Borrowed("always"))),
            _ => {}
        }
    }

    // Touch action
    if let Some(rest) = class.strip_prefix("touch-") {
        return matches!(
            rest,
            "auto"
                | "none"
                | "pan-x"
                | "pan-left"
                | "pan-right"
                | "pan-y"
                | "pan-up"
                | "pan-down"
                | "pinch-zoom"
                | "manipulation"
        )
        .then(|| tw("touch-action", Cow::Owned(rest.to_string())));
    }

    // User select
    if let Some(rest) = class.strip_prefix("select-") {
        return matches!(rest, "none" | "text" | "all" | "auto")
            .then(|| tw("user-select", Cow::Owned(rest.to_string())));
    }

    // Will change
    if let Some(rest) = class.strip_prefix("will-change-") {
        let value = match rest {
            "auto" => Cow::Borrowed("auto"),
            "scroll" => Cow::Borrowed("scroll-position"),
            "contents" => Cow::Borrowed("contents"),
            "transform" => Cow::Borrowed("transform"),
            _ => return None,
        };
        return Some(tw("will-change", value));
    }

    None
}

/// Parse SVG utilities (fill, stroke)
fn parse_svg_utility(class: &str) -> Option<(&'static str, Cow<'static, str>)> {
    // Fill
    if let Some(rest) = class.strip_prefix("fill-") {
        if rest == "none" {
            return Some(tw("fill", Cow::Borrowed("none")));
        }
        if let Some(color) = color_value(rest) {
            return Some(tw("fill", color));
        }
    }

    // Stroke
    if let Some(rest) = class.strip_prefix("stroke-") {
        if rest == "none" {
            return Some(tw("stroke", Cow::Borrowed("none")));
        }
        // Stroke width
        match rest {
            "0" => return Some(tw("stroke-width", Cow::Borrowed("0"))),
            "1" => return Some(tw("stroke-width", Cow::Borrowed("1"))),
            "2" => return Some(tw("stroke-width", Cow::Borrowed("2"))),
            _ => {}
        }
        // Stroke color
        if let Some(color) = color_value(rest) {
            return Some(tw("stroke", color));
        }
    }

    None
}

/// Cursors `cursor-*` takes
static CURSORS: phf::Set<&'static str> = phf_set! {
    "auto", "default", "pointer", "wait", "text", "move", "help", "not-allowed", "none",
    "context-menu", "progress", "cell", "crosshair", "vertical-text", "alias", "copy", "no-drop",
    "grab", "grabbing", "all-scroll", "col-resize", "row-resize", "n-resize", "e-resize",
    "s-resize", "w-resize", "ne-resize", "nw-resize", "se-resize", "sw-resize", "ew-resize",
    "ns-resize", "nesw-resize", "nwse-resize", "zoom-in", "zoom-out",
};

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::extract_style::extract_style_value::ExtractStyleValue;
    use css::class_map::reset_class_map;
    use css::file_map::reset_file_map;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;
    use std::collections::BTreeSet;

    // Helper to sort styles for consistent snapshots
    fn sort_styles(styles: Vec<ExtractStyleValue>) -> BTreeSet<ExtractStyleValue> {
        styles.into_iter().collect()
    }

    /// The styles of every class in `classes` that compiles
    fn parse_tailwind_to_styles(classes: &str) -> Vec<ExtractStyleValue> {
        classes
            .split_whitespace()
            .filter_map(parse_class)
            .flat_map(|class| {
                class
                    .styles()
                    .map(ExtractStyleValue::Static)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The declarations `class` compiles to, `None` when it stays as written
    fn declarations(class: &str) -> Option<Vec<(String, String)>> {
        parse_class(class).map(|class| {
            class
                .declarations
                .into_iter()
                .map(|(property, value)| (property.into_owned(), value.into_owned()))
                .collect()
        })
    }

    /// What `class` applies under: its level and each condition, written out
    fn conditions(class: &str) -> Option<(u8, Vec<String>)> {
        parse_class(class).map(|class| {
            (
                class.level,
                class
                    .conditions
                    .iter()
                    .map(|condition| {
                        condition
                            .as_ref()
                            .map_or_else(String::new, ToString::to_string)
                    })
                    .collect(),
            )
        })
    }

    struct Single {
        property: String,
        value: String,
    }

    /// The one declaration of a single-declaration utility
    fn parse_single_class(class: &str) -> Option<Single> {
        let mut declarations = declarations(class)?;
        assert_eq!(declarations.len(), 1, "{class}: {declarations:?}");
        let (property, value) = declarations.remove(0);
        Some(Single { property, value })
    }

    fn owned(declarations: &[(&str, &str)]) -> Vec<(String, String)> {
        declarations
            .iter()
            .map(|(property, value)| ((*property).to_string(), (*value).to_string()))
            .collect()
    }

    #[rstest]
    #[case("bg-red-500", "background-color", "oklch(63.7% 0.237 25.331)")]
    #[case("bg-blue-500", "background-color", "oklch(62.3% 0.214 259.815)")]
    #[case("bg-black", "background-color", "#000")]
    #[case("bg-white", "background-color", "#fff")]
    #[case("bg-transparent", "background-color", "transparent")]
    #[case("text-red-500", "color", "oklch(63.7% 0.237 25.331)")]
    #[case("text-white", "color", "#fff")]
    fn test_parse_color_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("p-4", "padding", "1rem")]
    #[case("p-0", "padding", "0px")]
    #[case("p-px", "padding", "1px")]
    #[case("p-0.5", "padding", "0.125rem")]
    #[case("px-4", "padding-inline", "1rem")]
    #[case("py-2", "padding-block", "0.5rem")]
    #[case("pt-4", "padding-top", "1rem")]
    #[case("pr-4", "padding-right", "1rem")]
    #[case("pb-4", "padding-bottom", "1rem")]
    #[case("pl-4", "padding-left", "1rem")]
    #[case("m-4", "margin", "1rem")]
    #[case("mx-auto", "margin-inline", "auto")]
    #[case("my-4", "margin-block", "1rem")]
    #[case("mt-4", "margin-top", "1rem")]
    fn test_parse_spacing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("w-full", "width", "100%")]
    #[case("w-screen", "width", "100vw")]
    #[case("w-auto", "width", "auto")]
    #[case("w-1/2", "width", "50%")]
    #[case("w-4", "width", "1rem")]
    #[case("h-full", "height", "100%")]
    #[case("h-screen", "height", "100vh")]
    #[case("min-w-0", "min-width", "0px")]
    #[case("min-w-full", "min-width", "100%")]
    #[case("max-w-sm", "max-width", "24rem")]
    #[case("max-w-xl", "max-width", "36rem")]
    fn test_parse_sizing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("flex", "display", "flex")]
    #[case("inline-flex", "display", "inline-flex")]
    #[case("grid", "display", "grid")]
    #[case("block", "display", "block")]
    #[case("hidden", "display", "none")]
    #[case("absolute", "position", "absolute")]
    #[case("relative", "position", "relative")]
    #[case("fixed", "position", "fixed")]
    #[case("sticky", "position", "sticky")]
    fn test_parse_layout_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("flex-row", "flex-direction", "row")]
    #[case("flex-col", "flex-direction", "column")]
    #[case("flex-wrap", "flex-wrap", "wrap")]
    #[case("flex-1", "flex", "1 1 0%")]
    #[case("justify-center", "justify-content", "center")]
    #[case("justify-between", "justify-content", "space-between")]
    #[case("items-center", "align-items", "center")]
    #[case("items-start", "align-items", "flex-start")]
    #[case("gap-4", "gap", "1rem")]
    fn test_parse_flex_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case(
        "font-sans",
        "font-family",
        "ui-sans-serif, system-ui, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'"
    )]
    #[case(
        "font-serif",
        "font-family",
        "ui-serif, Georgia, Cambria, 'Times New Roman', Times, serif"
    )]
    #[case(
        "font-mono",
        "font-family",
        "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace"
    )]
    #[case("font-bold", "font-weight", "700")]
    #[case("font-normal", "font-weight", "400")]
    #[case("text-center", "text-align", "center")]
    #[case("italic", "font-style", "italic")]
    #[case("underline", "text-decoration-line", "underline")]
    #[case("uppercase", "text-transform", "uppercase")]
    fn test_parse_typography_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("rounded", "border-radius", "0.25rem")]
    #[case("rounded-none", "border-radius", "0")]
    #[case("rounded-full", "border-radius", "calc(infinity * 1px)")]
    #[case("rounded-lg", "border-radius", "0.5rem")]
    #[case("border-red-500", "border-color", "oklch(63.7% 0.237 25.331)")]
    fn test_parse_border_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("opacity-50", "opacity", "0.5")]
    #[case("opacity-100", "opacity", "1")]
    #[case("opacity-0", "opacity", "0")]
    fn test_parse_opacity_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("z-10", "z-index", "10")]
    #[case("z-50", "z-index", "50")]
    #[case("z-auto", "z-index", "auto")]
    fn test_parse_z_index_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[test]
    fn test_parse_arbitrary_width() {
        let parsed = parse_single_class("w-[100px]").expect("Should parse");
        assert_eq!(parsed.property, "width");
        assert_eq!(parsed.value, "100px");
    }

    #[test]
    fn test_parse_arbitrary_color() {
        let parsed = parse_single_class("bg-[#ff0000]").expect("Should parse");
        assert_eq!(parsed.property, "background-color");
        assert_eq!(parsed.value, "#ff0000");
    }

    #[test]
    fn test_parse_arbitrary_calc() {
        let parsed = parse_single_class("w-[calc(100%-20px)]").expect("Should parse");
        assert_eq!(parsed.property, "width");
        assert_eq!(parsed.value, "calc(100% - 20px)");
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_basic() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles("bg-red-500 p-4 flex");
        assert_eq!(styles.len(), 3);

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_responsive() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles("sm:bg-blue-500 md:p-8 lg:flex");
        assert_eq!(styles.len(), 3);

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_variants() {
        reset_class_map();
        reset_file_map();

        let styles =
            parse_tailwind_to_styles("hover:bg-blue-500 focus:outline-none dark:text-white");

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_complex() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles(
            "flex items-center justify-between p-4 bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow duration-200",
        );

        assert_debug_snapshot!(sort_styles(styles));
    }

    #[test]
    fn test_parse_grid_utilities() {
        let parsed = parse_single_class("grid-cols-3").expect("Should parse");
        assert_eq!(parsed.property, "grid-template-columns");
        assert_eq!(parsed.value, "repeat(3, minmax(0, 1fr))");

        let parsed = parse_single_class("col-span-2").expect("Should parse");
        assert_eq!(parsed.property, "grid-column");
        assert_eq!(parsed.value, "span 2 / span 2");

        let parsed = parse_single_class("row-span-3").expect("Should parse");
        assert_eq!(parsed.property, "grid-row");
        assert_eq!(parsed.value, "span 3 / span 3");
    }

    #[test]
    fn test_parse_interactivity_utilities() {
        let parsed = parse_single_class("cursor-pointer").expect("Should parse");
        assert_eq!(parsed.property, "cursor");
        assert_eq!(parsed.value, "pointer");

        let parsed = parse_single_class("select-none").expect("Should parse");
        assert_eq!(parsed.property, "user-select");
        assert_eq!(parsed.value, "none");

        let parsed = parse_single_class("pointer-events-none").expect("Should parse");
        assert_eq!(parsed.property, "pointer-events");
        assert_eq!(parsed.value, "none");
    }

    #[test]
    fn test_parse_svg_utilities() {
        let parsed = parse_single_class("fill-red-500").expect("Should parse");
        assert_eq!(parsed.property, "fill");
        assert_eq!(parsed.value, "oklch(63.7% 0.237 25.331)");

        let parsed = parse_single_class("stroke-black").expect("Should parse");
        assert_eq!(parsed.property, "stroke");
        assert_eq!(parsed.value, "#000");

        let parsed = parse_single_class("stroke-2").expect("Should parse");
        assert_eq!(parsed.property, "stroke-width");
        assert_eq!(parsed.value, "2");
    }

    #[test]
    fn test_empty_string() {
        let styles = parse_tailwind_to_styles("");
        assert_eq!(styles, vec![]);
    }

    #[test]
    fn test_unknown_class() {
        let result = parse_single_class("unknown-class");
        assert!(result.is_none());
    }

    // ==================== WAVE 1: TailwindVariant Tests ====================

    // ==================== WAVE 2: Edge Cases & Arbitrary Values ====================

    // Wave 2.3: parse_arbitrary_value extended tests (lines 1057-1084)
    #[rstest]
    #[case("border-[#ff0000]", "border-color", "#ff0000")]
    #[case("opacity-[0.5]", "opacity", "0.5")]
    #[case("z-[999]", "z-index", "999")]
    #[case("font-[Arial]", "font-family", "Arial")]
    #[case("tracking-[0.2em]", "letter-spacing", "0.2em")]
    #[case("delay-[200ms]", "transition-delay", "200ms")]
    #[case("aspect-[16/9]", "aspect-ratio", "16/9")]
    #[case("columns-[3]", "columns", "3")]
    #[case("basis-[200px]", "flex-basis", "200px")]
    fn test_parse_arbitrary_values_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("scale-[1.5]", &[("scale", "1.5")])]
    #[case("rotate-[30deg]", &[("rotate", "30deg")])]
    #[case("translate-x-[50px]", &[("--tw-translate-x", "50px"), ("translate", TRANSLATE)])]
    #[case("translate-y-[50px]", &[("--tw-translate-y", "50px"), ("translate", TRANSLATE)])]
    #[case(
        "translate-[50px]",
        &[("--tw-translate-x", "50px"), ("--tw-translate-y", "50px"), ("translate", TRANSLATE)]
    )]
    #[case("scale-x-[2]", &[("--tw-scale-x", "2"), ("scale", SCALE)])]
    #[case("skew-x-[10deg]", &[("--tw-skew-x", "skewX(10deg)"), ("transform", TRANSFORM)])]
    #[case("skew-y-[10deg]", &[("--tw-skew-y", "skewY(10deg)"), ("transform", TRANSFORM)])]
    #[case(
        "skew-[10deg]",
        &[
            ("--tw-skew-x", "skewX(10deg)"),
            ("--tw-skew-y", "skewY(10deg)"),
            ("transform", TRANSFORM),
        ]
    )]
    #[case("-translate-x-[13px]", &[("--tw-translate-x", "calc(13px * -1)"), ("translate", TRANSLATE)])]
    #[case("-skew-y-[6deg]", &[("--tw-skew-y", "skewY(calc(6deg * -1))"), ("transform", TRANSFORM)])]
    fn test_parse_arbitrary_transform_values(
        #[case] class: &str,
        #[case] expected: &[(&str, &str)],
    ) {
        assert_eq!(declarations(class), Some(owned(expected)));
    }

    #[rstest]
    #[case("grid-cols-[200px_1fr]", "grid-template-columns", "200px 1fr")]
    #[case("grid-rows-[auto_1fr]", "grid-template-rows", "auto 1fr")]
    #[case("col-span-[2]", "grid-column", "span 2 / span 2")]
    #[case("row-span-[3]", "grid-row", "span 3 / span 3")]
    fn test_parse_arbitrary_grid_values(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 3: Layout & Flex/Grid Utilities ====================

    // Wave 3.1: aspect-ratio utilities (lines 1198-1204)
    #[rstest]
    #[case("aspect-auto", "aspect-ratio", "auto")]
    #[case("aspect-square", "aspect-ratio", "1 / 1")]
    #[case("aspect-video", "aspect-ratio", "16 / 9")]
    fn test_parse_aspect_ratio_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.2: columns utilities (lines 1209-1226)
    #[rstest]
    #[case("columns-auto", "columns", "auto")]
    #[case("columns-3xs", "columns", "16rem")]
    #[case("columns-2xs", "columns", "18rem")]
    #[case("columns-xs", "columns", "20rem")]
    #[case("columns-sm", "columns", "24rem")]
    #[case("columns-md", "columns", "28rem")]
    #[case("columns-lg", "columns", "32rem")]
    #[case("columns-xl", "columns", "36rem")]
    #[case("columns-2xl", "columns", "42rem")]
    #[case("columns-3xl", "columns", "48rem")]
    #[case("columns-4xl", "columns", "56rem")]
    #[case("columns-5xl", "columns", "64rem")]
    #[case("columns-6xl", "columns", "72rem")]
    #[case("columns-7xl", "columns", "80rem")]
    fn test_parse_columns_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.3: break utilities (lines 1231-1252)
    #[rstest]
    #[case("break-after-auto", "break-after", "auto")]
    #[case("break-after-avoid", "break-after", "avoid")]
    #[case("break-after-all", "break-after", "all")]
    #[case("break-after-avoid-page", "break-after", "avoid-page")]
    #[case("break-after-page", "break-after", "page")]
    #[case("break-after-left", "break-after", "left")]
    #[case("break-after-right", "break-after", "right")]
    #[case("break-after-column", "break-after", "column")]
    #[case("break-before-auto", "break-before", "auto")]
    #[case("break-before-avoid", "break-before", "avoid")]
    #[case("break-before-all", "break-before", "all")]
    #[case("break-before-avoid-page", "break-before", "avoid-page")]
    #[case("break-before-page", "break-before", "page")]
    #[case("break-before-left", "break-before", "left")]
    #[case("break-before-right", "break-before", "right")]
    #[case("break-before-column", "break-before", "column")]
    #[case("break-inside-auto", "break-inside", "auto")]
    #[case("break-inside-avoid", "break-inside", "avoid")]
    #[case("break-inside-avoid-page", "break-inside", "avoid-page")]
    #[case("break-inside-avoid-column", "break-inside", "avoid-column")]
    fn test_parse_break_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.4: Position utilities (lines 1273-1304)
    #[rstest]
    #[case("top-0", "top", "0px")]
    #[case("top-4", "top", "1rem")]
    #[case("right-0", "right", "0px")]
    #[case("right-4", "right", "1rem")]
    #[case("bottom-0", "bottom", "0px")]
    #[case("bottom-4", "bottom", "1rem")]
    #[case("left-0", "left", "0px")]
    #[case("left-4", "left", "1rem")]
    #[case("inset-0", "inset", "0px")]
    #[case("inset-4", "inset", "1rem")]
    #[case("inset-x-0", "inset-inline", "0px")]
    #[case("inset-x-4", "inset-inline", "1rem")]
    #[case("inset-y-0", "inset-block", "0px")]
    #[case("inset-y-4", "inset-block", "1rem")]
    fn test_parse_position_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 3.5: Flex/Grid extended utilities (lines 1463-1529)
    #[rstest]
    #[case("basis-4", "flex-basis", "1rem")]
    #[case("basis-8", "flex-basis", "2rem")]
    #[case("order-1", "order", "1")]
    #[case("order-12", "order", "12")]
    #[case("grid-cols-1", "grid-template-columns", "repeat(1, minmax(0, 1fr))")]
    #[case("grid-cols-12", "grid-template-columns", "repeat(12, minmax(0, 1fr))")]
    #[case("grid-rows-1", "grid-template-rows", "repeat(1, minmax(0, 1fr))")]
    #[case("grid-rows-6", "grid-template-rows", "repeat(6, minmax(0, 1fr))")]
    #[case("col-start-1", "grid-column-start", "1")]
    #[case("col-start-auto", "grid-column-start", "auto")]
    #[case("col-end-1", "grid-column-end", "1")]
    #[case("col-end-auto", "grid-column-end", "auto")]
    #[case("row-start-1", "grid-row-start", "1")]
    #[case("row-start-auto", "grid-row-start", "auto")]
    #[case("row-end-1", "grid-row-end", "1")]
    #[case("row-end-auto", "grid-row-end", "auto")]
    #[case("gap-x-4", "column-gap", "1rem")]
    #[case("gap-y-4", "row-gap", "1rem")]
    fn test_parse_flex_grid_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 4: Spacing, Sizing & Typography ====================

    // Wave 4.1: Logical spacing utilities (lines 1580-1656)
    #[rstest]
    #[case("ps-4", "padding-inline-start", "1rem")]
    #[case("pe-4", "padding-inline-end", "1rem")]
    #[case("ms-4", "margin-inline-start", "1rem")]
    #[case("me-4", "margin-inline-end", "1rem")]
    #[case("mr-4", "margin-right", "1rem")]
    #[case("mb-4", "margin-bottom", "1rem")]
    #[case("ml-4", "margin-left", "1rem")]
    #[case("space-x-reverse", "--tw-space-x-reverse", "1")]
    #[case("space-y-reverse", "--tw-space-y-reverse", "1")]
    fn test_parse_logical_spacing_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 4.2: Sizing variants (lines 1677-1789)
    #[rstest]
    #[case("min-w-min", "min-width", "min-content")]
    #[case("min-w-max", "min-width", "max-content")]
    #[case("min-w-fit", "min-width", "fit-content")]
    #[case("max-w-2xl", "max-width", "42rem")]
    #[case("max-w-3xl", "max-width", "48rem")]
    #[case("max-w-4xl", "max-width", "56rem")]
    #[case("max-w-5xl", "max-width", "64rem")]
    #[case("max-w-6xl", "max-width", "72rem")]
    #[case("max-w-7xl", "max-width", "80rem")]
    #[case("max-w-min", "max-width", "min-content")]
    #[case("max-w-max", "max-width", "max-content")]
    #[case("max-w-fit", "max-width", "fit-content")]
    #[case("max-w-prose", "max-width", "65ch")]
    #[case("max-w-screen-sm", "max-width", "640px")]
    #[case("max-w-screen-md", "max-width", "768px")]
    #[case("max-w-screen-lg", "max-width", "1024px")]
    #[case("max-w-screen-xl", "max-width", "1280px")]
    #[case("max-w-screen-2xl", "max-width", "1536px")]
    fn test_parse_width_utilities_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("h-svh", "height", "100svh")]
    #[case("h-lvh", "height", "100lvh")]
    #[case("h-dvh", "height", "100dvh")]
    #[case("min-h-svh", "min-height", "100svh")]
    #[case("min-h-lvh", "min-height", "100lvh")]
    #[case("min-h-dvh", "min-height", "100dvh")]
    #[case("min-h-min", "min-height", "min-content")]
    #[case("min-h-max", "min-height", "max-content")]
    #[case("min-h-fit", "min-height", "fit-content")]
    #[case("max-h-svh", "max-height", "100svh")]
    #[case("max-h-lvh", "max-height", "100lvh")]
    #[case("max-h-dvh", "max-height", "100dvh")]
    #[case("max-h-min", "max-height", "min-content")]
    #[case("max-h-max", "max-height", "max-content")]
    #[case("max-h-fit", "max-height", "fit-content")]
    fn test_parse_height_utilities_extended(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 4.3: Typography extended (lines 1829-1940)
    #[rstest]
    #[case("text-start", "text-align", "start")]
    #[case("text-end", "text-align", "end")]
    #[case("hyphens-none", "hyphens", "none")]
    #[case("hyphens-manual", "hyphens", "manual")]
    #[case("hyphens-auto", "hyphens", "auto")]
    #[case("tracking-tighter", "letter-spacing", "-0.05em")]
    #[case("tracking-tight", "letter-spacing", "-0.025em")]
    #[case("tracking-normal", "letter-spacing", "0em")]
    #[case("tracking-wide", "letter-spacing", "0.025em")]
    #[case("tracking-wider", "letter-spacing", "0.05em")]
    #[case("tracking-widest", "letter-spacing", "0.1em")]
    fn test_parse_typography_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // `leading-*` also sets `--tw-leading`, which wins over a font size's line height
    #[rstest]
    #[case("leading-none", "1")]
    #[case("leading-tight", "1.25")]
    #[case("leading-snug", "1.375")]
    #[case("leading-normal", "1.5")]
    #[case("leading-relaxed", "1.625")]
    #[case("leading-loose", "2")]
    #[case("leading-3", "0.75rem")]
    #[case("leading-4", "1rem")]
    #[case("leading-5", "1.25rem")]
    #[case("leading-6", "1.5rem")]
    #[case("leading-7", "1.75rem")]
    #[case("leading-8", "2rem")]
    #[case("leading-9", "2.25rem")]
    #[case("leading-10", "2.5rem")]
    #[case("leading-[2]", "2")]
    #[case("leading-(--l)", "var(--l)")]
    fn test_parse_leading_utilities(#[case] class: &str, #[case] expected_value: &str) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                ("--tw-leading", expected_value),
                ("line-height", expected_value)
            ]))
        );
    }

    // Wave 4.4: List styles & alignment (lines 1947-1965)
    #[rstest]
    #[case("list-inside", "list-style-position", "inside")]
    #[case("list-outside", "list-style-position", "outside")]
    #[case("list-none", "list-style-type", "none")]
    #[case("list-disc", "list-style-type", "disc")]
    #[case("list-decimal", "list-style-type", "decimal")]
    #[case("align-baseline", "vertical-align", "baseline")]
    #[case("align-top", "vertical-align", "top")]
    #[case("align-middle", "vertical-align", "middle")]
    #[case("align-bottom", "vertical-align", "bottom")]
    #[case("content-none", "content", "none")]
    fn test_parse_list_align_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ==================== WAVE 5: Backgrounds, Borders, Effects, etc. ====================

    // Wave 5.1: Background utilities (lines 1981-2051)
    #[rstest]
    #[case("bg-fixed", "background-attachment", "fixed")]
    #[case("bg-local", "background-attachment", "local")]
    #[case("bg-scroll", "background-attachment", "scroll")]
    #[case("bg-clip-border", "background-clip", "border-box")]
    #[case("bg-clip-padding", "background-clip", "padding-box")]
    #[case("bg-clip-content", "background-clip", "content-box")]
    #[case("bg-clip-text", "background-clip", "text")]
    #[case("bg-origin-border", "background-origin", "border-box")]
    #[case("bg-origin-padding", "background-origin", "padding-box")]
    #[case("bg-origin-content", "background-origin", "content-box")]
    fn test_parse_background_attachment_clip_origin(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("bg-bottom", "background-position", "bottom")]
    #[case("bg-center", "background-position", "center")]
    #[case("bg-left", "background-position", "left")]
    #[case("bg-left-bottom", "background-position", "left bottom")]
    #[case("bg-left-top", "background-position", "left top")]
    #[case("bg-right", "background-position", "right")]
    #[case("bg-right-bottom", "background-position", "right bottom")]
    #[case("bg-right-top", "background-position", "right top")]
    #[case("bg-top", "background-position", "top")]
    fn test_parse_background_position(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("bg-repeat", "background-repeat", "repeat")]
    #[case("bg-no-repeat", "background-repeat", "no-repeat")]
    #[case("bg-repeat-x", "background-repeat", "repeat-x")]
    #[case("bg-repeat-y", "background-repeat", "repeat-y")]
    #[case("bg-repeat-round", "background-repeat", "round")]
    #[case("bg-repeat-space", "background-repeat", "space")]
    #[case("bg-auto", "background-size", "auto")]
    #[case("bg-cover", "background-size", "cover")]
    #[case("bg-contain", "background-size", "contain")]
    #[case("bg-none", "background-image", "none")]
    fn test_parse_background_repeat_size(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Sides round both of their corners
    #[rstest]
    #[case("rounded-t-lg", &[("border-top-left-radius", "0.5rem"), ("border-top-right-radius", "0.5rem")])]
    #[case("rounded-r-lg", &[("border-top-right-radius", "0.5rem"), ("border-bottom-right-radius", "0.5rem")])]
    #[case("rounded-b-lg", &[("border-bottom-right-radius", "0.5rem"), ("border-bottom-left-radius", "0.5rem")])]
    #[case("rounded-l-lg", &[("border-top-left-radius", "0.5rem"), ("border-bottom-left-radius", "0.5rem")])]
    #[case("rounded-s-lg", &[("border-start-start-radius", "0.5rem"), ("border-end-start-radius", "0.5rem")])]
    #[case("rounded-e-lg", &[("border-start-end-radius", "0.5rem"), ("border-end-end-radius", "0.5rem")])]
    #[case("rounded-t", &[("border-top-left-radius", "0.25rem"), ("border-top-right-radius", "0.25rem")])]
    #[case("rounded-tl-lg", &[("border-top-left-radius", "0.5rem")])]
    #[case("rounded-tr-lg", &[("border-top-right-radius", "0.5rem")])]
    #[case("rounded-br-lg", &[("border-bottom-right-radius", "0.5rem")])]
    #[case("rounded-bl-lg", &[("border-bottom-left-radius", "0.5rem")])]
    #[case("rounded-ss-lg", &[("border-start-start-radius", "0.5rem")])]
    #[case("rounded-se-lg", &[("border-start-end-radius", "0.5rem")])]
    #[case("rounded-ee-lg", &[("border-end-end-radius", "0.5rem")])]
    #[case("rounded-es-lg", &[("border-end-start-radius", "0.5rem")])]
    #[case("rounded-t-[3px]", &[("border-top-left-radius", "3px"), ("border-top-right-radius", "3px")])]
    #[case("rounded-[3px]", &[("border-radius", "3px")])]
    fn test_parse_border_radius_corners(#[case] class: &str, #[case] expected: &[(&str, &str)]) {
        assert_eq!(declarations(class), Some(owned(expected)));
    }

    // Wave 5.4: Border styles, outline, ring, divide (lines 2211-2313)
    // Wave 5.5: Effects (lines 2328-2350)
    #[rstest]
    #[case("bg-blend-normal", "background-blend-mode", "normal")]
    #[case("bg-blend-multiply", "background-blend-mode", "multiply")]
    #[case("bg-blend-screen", "background-blend-mode", "screen")]
    #[case("bg-blend-overlay", "background-blend-mode", "overlay")]
    fn test_parse_blend_mode_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.6: Filters & backdrop filters (lines 2365-2555)
    // Wave 5.7: Transitions & animations (lines 2600-2618)
    #[rstest]
    #[case("animate-none", "animation", "none")]
    #[case("animate-spin", "animation", "spin 1s linear infinite")]
    #[case(
        "animate-ping",
        "animation",
        "ping 1s cubic-bezier(0, 0, 0.2, 1) infinite"
    )]
    #[case(
        "animate-pulse",
        "animation",
        "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite"
    )]
    #[case("animate-bounce", "animation", "bounce 1s infinite")]
    fn test_parse_animation_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("scale-x-0", "--tw-scale-x", "0%")]
    #[case("scale-x-50", "--tw-scale-x", "50%")]
    #[case("scale-x-100", "--tw-scale-x", "100%")]
    #[case("scale-x-150", "--tw-scale-x", "150%")]
    #[case("scale-y-0", "--tw-scale-y", "0%")]
    #[case("scale-y-50", "--tw-scale-y", "50%")]
    #[case("scale-y-100", "--tw-scale-y", "100%")]
    #[case("scale-y-150", "--tw-scale-y", "150%")]
    #[case("-scale-x-50", "--tw-scale-x", "-50%")]
    fn test_parse_scale_axis_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        assert_eq!(
            declarations(class),
            Some(owned(&[(expected_prop, expected_value), ("scale", SCALE)]))
        );
    }

    #[rstest]
    #[case("scale-110", "110%")]
    #[case("-scale-110", "-110%")]
    fn test_parse_scale_utilities(#[case] class: &str, #[case] expected_value: &str) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                ("--tw-scale-x", expected_value),
                ("--tw-scale-y", expected_value),
                ("--tw-scale-z", expected_value),
                ("scale", SCALE),
            ]))
        );
    }

    #[rstest]
    #[case("rotate-0", "0deg")]
    #[case("rotate-1", "1deg")]
    #[case("rotate-2", "2deg")]
    #[case("rotate-3", "3deg")]
    #[case("rotate-6", "6deg")]
    #[case("rotate-12", "12deg")]
    #[case("rotate-90", "90deg")]
    #[case("rotate-180", "180deg")]
    #[case("rotate-15", "15deg")]
    #[case("-rotate-45", "-45deg")]
    fn test_parse_rotate_utilities(#[case] class: &str, #[case] expected_value: &str) {
        assert_eq!(
            declarations(class),
            Some(owned(&[("rotate", expected_value)]))
        );
    }

    #[rstest]
    #[case("translate-y-4", "--tw-translate-y", "1rem")]
    #[case("translate-y-px", "--tw-translate-y", "1px")]
    #[case("translate-y-full", "--tw-translate-y", "100%")]
    #[case("translate-y-1/2", "--tw-translate-y", "50%")]
    #[case("translate-x-4", "--tw-translate-x", "1rem")]
    #[case("-translate-x-4", "--tw-translate-x", "-1rem")]
    #[case("-translate-x-1/2", "--tw-translate-x", "-50%")]
    fn test_parse_translate_y_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                (expected_prop, expected_value),
                ("translate", TRANSLATE)
            ]))
        );
    }

    #[test]
    fn test_parse_translate_both_axes() {
        assert_eq!(
            declarations("translate-2"),
            Some(owned(&[
                ("--tw-translate-x", "0.5rem"),
                ("--tw-translate-y", "0.5rem"),
                ("translate", TRANSLATE),
            ]))
        );
    }

    #[rstest]
    #[case("skew-x-0", "--tw-skew-x", "skewX(0deg)")]
    #[case("skew-x-1", "--tw-skew-x", "skewX(1deg)")]
    #[case("skew-x-2", "--tw-skew-x", "skewX(2deg)")]
    #[case("skew-x-3", "--tw-skew-x", "skewX(3deg)")]
    #[case("skew-x-6", "--tw-skew-x", "skewX(6deg)")]
    #[case("skew-x-12", "--tw-skew-x", "skewX(12deg)")]
    #[case("skew-y-0", "--tw-skew-y", "skewY(0deg)")]
    #[case("skew-y-1", "--tw-skew-y", "skewY(1deg)")]
    #[case("skew-y-2", "--tw-skew-y", "skewY(2deg)")]
    #[case("skew-y-3", "--tw-skew-y", "skewY(3deg)")]
    #[case("skew-y-6", "--tw-skew-y", "skewY(6deg)")]
    #[case("skew-y-12", "--tw-skew-y", "skewY(12deg)")]
    #[case("-skew-y-6", "--tw-skew-y", "skewY(-6deg)")]
    fn test_parse_skew_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                (expected_prop, expected_value),
                ("transform", TRANSFORM)
            ]))
        );
    }

    #[test]
    fn test_parse_skew_both_axes() {
        assert_eq!(
            declarations("skew-3"),
            Some(owned(&[
                ("--tw-skew-x", "skewX(3deg)"),
                ("--tw-skew-y", "skewY(3deg)"),
                ("transform", TRANSFORM),
            ]))
        );
    }

    #[rstest]
    #[case("origin-center", "transform-origin", "center")]
    #[case("origin-top", "transform-origin", "top")]
    #[case("origin-top-right", "transform-origin", "top right")]
    #[case("origin-right", "transform-origin", "right")]
    #[case("origin-bottom-right", "transform-origin", "bottom right")]
    #[case("origin-bottom", "transform-origin", "bottom")]
    #[case("origin-bottom-left", "transform-origin", "bottom left")]
    #[case("origin-left", "transform-origin", "left")]
    #[case("origin-top-left", "transform-origin", "top left")]
    fn test_parse_transform_origin_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.9: Interactivity (lines 2732-2842)
    #[rstest]
    #[case("accent-auto", "accent-color", "auto")]
    #[case("accent-red-500", "accent-color", "oklch(63.7% 0.237 25.331)")]
    #[case("appearance-none", "appearance", "none")]
    #[case("appearance-auto", "appearance", "auto")]
    #[case("caret-red-500", "caret-color", "oklch(63.7% 0.237 25.331)")]
    fn test_parse_interactivity_accent_caret(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("scroll-auto", "scroll-behavior", "auto")]
    #[case("scroll-smooth", "scroll-behavior", "smooth")]
    #[case("scroll-m-4", "scroll-margin", "1rem")]
    #[case("scroll-p-4", "scroll-padding", "1rem")]
    fn test_parse_scroll_behavior_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("snap-start", "scroll-snap-align", "start")]
    #[case("snap-end", "scroll-snap-align", "end")]
    #[case("snap-center", "scroll-snap-align", "center")]
    #[case("snap-align-none", "scroll-snap-align", "none")]
    #[case("snap-none", "scroll-snap-type", "none")]
    #[case("snap-x", "scroll-snap-type", "x var(--tw-scroll-snap-strictness)")]
    #[case("snap-y", "scroll-snap-type", "y var(--tw-scroll-snap-strictness)")]
    #[case(
        "snap-both",
        "scroll-snap-type",
        "both var(--tw-scroll-snap-strictness)"
    )]
    #[case("snap-mandatory", "--tw-scroll-snap-strictness", "mandatory")]
    #[case("snap-proximity", "--tw-scroll-snap-strictness", "proximity")]
    #[case("snap-normal", "scroll-snap-stop", "normal")]
    #[case("snap-always", "scroll-snap-stop", "always")]
    fn test_parse_snap_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("touch-auto", "touch-action", "auto")]
    #[case("touch-none", "touch-action", "none")]
    #[case("touch-pan-x", "touch-action", "pan-x")]
    #[case("touch-pan-y", "touch-action", "pan-y")]
    #[case("touch-manipulation", "touch-action", "manipulation")]
    fn test_parse_touch_action_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("will-change-auto", "will-change", "auto")]
    #[case("will-change-scroll", "will-change", "scroll-position")]
    #[case("will-change-contents", "will-change", "contents")]
    #[case("will-change-transform", "will-change", "transform")]
    fn test_parse_will_change_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 5.10: SVG utilities (lines 2853-2863)
    #[rstest]
    #[case("fill-none", "fill", "none")]
    #[case("stroke-none", "stroke", "none")]
    #[case("stroke-0", "stroke-width", "0")]
    #[case("stroke-1", "stroke-width", "1")]
    fn test_parse_svg_extended_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // ============================================================================
    // WAVE 6: Coverage Gap Tests - Remaining Uncovered Lines
    // ============================================================================

    #[rstest]
    #[case("aspect-4/3", "aspect-ratio", "4/3")]
    #[case("aspect-16/10", "aspect-ratio", "16/10")]
    #[case("aspect-21/9", "aspect-ratio", "21/9")]
    #[case("aspect-3/2", "aspect-ratio", "3/2")]
    fn test_parse_custom_aspect_ratio(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse custom aspect ratio");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.4: Numeric columns fallback (line 1224)
    #[rstest]
    #[case("columns-1", "columns", "1")]
    #[case("columns-2", "columns", "2")]
    #[case("columns-3", "columns", "3")]
    #[case("columns-4", "columns", "4")]
    #[case("columns-5", "columns", "5")]
    #[case("columns-6", "columns", "6")]
    #[case("columns-12", "columns", "12")]
    fn test_parse_numeric_columns(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse numeric columns");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.5: Box decoration break (lines 1258, 1261)
    #[rstest]
    #[case("box-decoration-clone", "box-decoration-break", "clone")]
    #[case("box-decoration-slice", "box-decoration-break", "slice")]
    fn test_parse_box_decoration_break(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse box-decoration-break");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.6: min-w/max-w/h fallback paths (lines 1681-1785)
    #[test]
    fn test_min_w_unknown_value_returns_none() {
        // min-w- with unknown value should return None (line 1684)
        let result = parse_single_class("min-w-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_max_w_unknown_value_returns_none() {
        // max-w- with unknown value should return None (line 1721)
        let result = parse_single_class("max-w-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_h_unknown_value_returns_none() {
        // h- with unknown value should return None (line 1739)
        let result = parse_single_class("h-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_min_h_unknown_value_returns_none() {
        // min-h- with unknown value should return None (line 1762)
        let result = parse_single_class("min-h-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_max_h_unknown_value_returns_none() {
        // max-h- with unknown value should return None (line 1785)
        let result = parse_single_class("max-h-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_size_unknown_value_returns_none() {
        // size- with unknown value should return None (line 1797)
        let result = parse_single_class("size-unknown");
        assert!(result.is_none());
    }

    // Wave 6.7: Typography edge cases (lines 1833-1892)
    #[rstest]
    #[case("line-through", "text-decoration-line", "line-through")]
    fn test_parse_line_through(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse line-through");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[test]
    fn test_parse_truncate() {
        assert_eq!(
            declarations("truncate"),
            Some(owned(&[
                ("overflow", "hidden"),
                ("text-overflow", "ellipsis"),
                ("white-space", "nowrap"),
            ]))
        );
    }

    #[rstest]
    #[case("whitespace-normal", "white-space", "normal")]
    #[case("whitespace-nowrap", "white-space", "nowrap")]
    #[case("whitespace-pre", "white-space", "pre")]
    #[case("whitespace-pre-line", "white-space", "pre-line")]
    #[case("whitespace-pre-wrap", "white-space", "pre-wrap")]
    #[case("whitespace-break-spaces", "white-space", "break-spaces")]
    fn test_parse_whitespace_utilities(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse whitespace");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Only Tailwind's names compile; any other word is someone else's class
    #[rstest]
    #[case("tracking-custom")]
    #[case("tracking-0.5em")]
    fn test_parse_tracking_fallback(#[case] class: &str) {
        assert_eq!(declarations(class), None);
    }

    #[test]
    fn test_parse_leading_fallback() {
        assert_eq!(
            declarations("leading-1.5"),
            Some(owned(&[
                ("--tw-leading", "0.375rem"),
                ("line-height", "0.375rem")
            ]))
        );
    }

    // Wave 6.9: List style fallback (line 1953)
    #[test]
    fn test_list_style_unknown_returns_none() {
        // list- with unknown value should not match (line 1953)
        let result = parse_single_class("list-unknown");
        assert!(result.is_none());
    }

    // Wave 6.10: Gradient direction fallback (line 2049)
    #[test]
    fn test_bg_gradient_unknown_direction_returns_none() {
        // bg-gradient-to- with unknown direction should return None
        let result = parse_single_class("bg-gradient-to-xyz");
        assert!(result.is_none());
    }

    // Wave 6.11: Individual rounded variants (via BORDER_RADIUS_SCALE lookup)
    #[rstest]
    #[case("rounded-none", "border-radius", "0")]
    #[case("rounded-sm", "border-radius", "0.25rem")]
    #[case("rounded-md", "border-radius", "0.375rem")]
    #[case("rounded-lg", "border-radius", "0.5rem")]
    #[case("rounded-xl", "border-radius", "0.75rem")]
    #[case("rounded-2xl", "border-radius", "1rem")]
    #[case("rounded-3xl", "border-radius", "1.5rem")]
    #[case("rounded-full", "border-radius", "calc(infinity * 1px)")]
    fn test_parse_individual_rounded_variants(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse rounded variant");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.12: Border collapse/separate (border-0/2/4/8 handled via BORDER_WIDTH_SCALE)
    #[rstest]
    #[case("border-collapse", "border-collapse", "collapse")]
    #[case("border-separate", "border-collapse", "separate")]
    fn test_parse_border_collapse_separate(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse border collapse/separate");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.13: Outline color (lines 2250-2251)
    #[rstest]
    #[case("outline-black", "outline-color", "#000")]
    #[case("outline-white", "outline-color", "#fff")]
    #[case("outline-red-500", "outline-color", "oklch(63.7% 0.237 25.331)")]
    #[case("outline-blue-500", "outline-color", "oklch(62.3% 0.214 259.815)")]
    fn test_parse_outline_color(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse outline color");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.14: Ring utilities (lines 2273-2281)
    #[rstest]
    #[case("ring-black", "--tw-ring-color", "#000")]
    #[case("ring-white", "--tw-ring-color", "#fff")]
    #[case("ring-red-500", "--tw-ring-color", "oklch(63.7% 0.237 25.331)")]
    #[case("ring-blue-500", "--tw-ring-color", "oklch(62.3% 0.214 259.815)")]
    fn test_parse_ring_color(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse ring color");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 6.15: Divide utilities (lines 2293-2313)
    // Wave 6.16: Filter fallbacks (lines 2390, 2405, 2419)
    #[test]
    fn test_drop_shadow_unknown_value_returns_none() {
        let result = parse_single_class("drop-shadow-unknown");
        assert!(result.is_none());
        let result = parse_single_class("drop-shadow-huge");
        assert!(result.is_none());
    }

    // Wave 6.17: Backdrop filter extended values (lines 2504-2555)
    #[test]
    fn test_backdrop_brightness_unknown_returns_none() {
        let result = parse_single_class("backdrop-brightness-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_backdrop_contrast_unknown_returns_none() {
        let result = parse_single_class("backdrop-contrast-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_backdrop_saturate_unknown_returns_none() {
        let result = parse_single_class("backdrop-saturate-unknown");
        assert!(result.is_none());
    }

    #[test]
    fn test_rotate_unknown_value_returns_none() {
        assert!(parse_single_class("rotate-unknown").is_none());
        // 3D rotation is not compiled
    }

    #[test]
    fn test_skew_x_unknown_value_returns_none() {
        assert!(parse_single_class("skew-x-unknown").is_none());
        assert!(parse_single_class("skew-x-1px").is_none());
    }

    #[test]
    fn test_skew_y_unknown_value_returns_none() {
        assert!(parse_single_class("skew-y-unknown").is_none());
        assert!(parse_single_class("skew-y-1px").is_none());
    }

    #[test]
    fn test_origin_unknown_value_returns_none() {
        let result = parse_single_class("origin-unknown");
        assert!(result.is_none());
        let result = parse_single_class("origin-middle");
        assert!(result.is_none());
    }

    // Wave 6.19: Snap/will-change fallbacks (lines 2819, 2840)
    #[test]
    fn test_snap_unknown_value_returns_none() {
        let result = parse_single_class("snap-unknown");
        assert!(result.is_none());
        let result = parse_single_class("snap-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_will_change_unknown_value_returns_none() {
        let result = parse_single_class("will-change-unknown");
        assert!(result.is_none());
        let result = parse_single_class("will-change-xyz");
        assert!(result.is_none());
    }

    // Wave 6.20: Break utilities fallback (line 1252)
    #[test]
    fn test_break_unknown_value_returns_none() {
        let result = parse_single_class("break-unknown");
        assert!(result.is_none());
        let result = parse_single_class("break-xyz");
        assert!(result.is_none());
    }

    // Wave 6.21: Backdrop blur unknown value (line 2488)
    #[test]
    fn test_backdrop_blur_unknown_returns_none() {
        let result = parse_single_class("backdrop-blur-unknown");
        assert!(result.is_none());
        let result = parse_single_class("backdrop-blur-huge");
        assert!(result.is_none());
    }

    // ============================================================================
    // WAVE 7: Additional Coverage Gap Tests
    // ============================================================================

    // Wave 7.3: min-w/max-w/min-h/max-h/size with SPACING_SCALE values (lines 1682, 1719, 1760, 1783, 1797)
    #[rstest]
    #[case("min-w-4", "min-width", "1rem")]
    #[case("min-w-8", "min-width", "2rem")]
    #[case("min-w-12", "min-width", "3rem")]
    #[case("min-w-px", "min-width", "1px")]
    #[case("min-w-0.5", "min-width", "0.125rem")]
    fn test_parse_min_w_spacing_scale(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse min-w spacing");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("max-w-4", "max-width", "1rem")]
    #[case("max-w-8", "max-width", "2rem")]
    #[case("max-w-12", "max-width", "3rem")]
    #[case("max-w-px", "max-width", "1px")]
    fn test_parse_max_w_spacing_scale(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse max-w spacing");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("min-h-4", "min-height", "1rem")]
    #[case("min-h-8", "min-height", "2rem")]
    #[case("min-h-12", "min-height", "3rem")]
    #[case("min-h-px", "min-height", "1px")]
    fn test_parse_min_h_spacing_scale(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse min-h spacing");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("max-h-4", "max-height", "1rem")]
    #[case("max-h-8", "max-height", "2rem")]
    #[case("max-h-12", "max-height", "3rem")]
    #[case("max-h-px", "max-height", "1px")]
    fn test_parse_max_h_spacing_scale(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse max-h spacing");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    #[rstest]
    #[case("size-4", "1rem")]
    #[case("size-8", "2rem")]
    #[case("size-12", "3rem")]
    #[case("size-px", "1px")]
    #[case("size-0.5", "0.125rem")]
    #[case("size-full", "100%")]
    #[case("size-[3px]", "3px")]
    fn test_parse_size_spacing_scale(#[case] class: &str, #[case] expected_value: &str) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                ("width", expected_value),
                ("height", expected_value)
            ]))
        );
    }

    // Wave 7.4: text- prefix fallback when not font-size or text-align (line 1833)
    #[rstest]
    #[case("text-unknown")]
    #[case("text-foo")]
    #[case("text-bar")]
    fn test_text_prefix_unknown_returns_none(#[case] class: &str) {
        let result = parse_single_class(class);
        assert!(result.is_none());
    }

    // Wave 7.5: Individual rounded variants via BORDER_RADIUS_SCALE lookup
    #[rstest]
    #[case("rounded", "border-radius", "0.25rem")]
    #[case("rounded-none", "border-radius", "0")]
    #[case("rounded-sm", "border-radius", "0.25rem")]
    #[case("rounded-md", "border-radius", "0.375rem")]
    #[case("rounded-lg", "border-radius", "0.5rem")]
    #[case("rounded-xl", "border-radius", "0.75rem")]
    #[case("rounded-2xl", "border-radius", "1rem")]
    #[case("rounded-3xl", "border-radius", "1.5rem")]
    #[case("rounded-full", "border-radius", "calc(infinity * 1px)")]
    fn test_rounded_variants_full_path(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse rounded");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 7.6: border-0/2/4/8 via BORDER_WIDTH_SCALE lookup
    // Wave 7.7: divide- unknown value fallback (line 2313)
    #[rstest]
    #[case("divide-unknown")]
    #[case("divide-xyz")]
    fn test_divide_unknown_returns_none(#[case] class: &str) {
        let result = parse_single_class(class);
        assert!(result.is_none());
    }

    // Wave 7.8: shadow without suffix (line 2333)
    // Wave 7.9: mix-blend- prefix (line 2345)
    #[rstest]
    #[case("mix-blend-normal", "mix-blend-mode", "normal")]
    #[case("mix-blend-multiply", "mix-blend-mode", "multiply")]
    #[case("mix-blend-screen", "mix-blend-mode", "screen")]
    #[case("mix-blend-overlay", "mix-blend-mode", "overlay")]
    #[case("mix-blend-darken", "mix-blend-mode", "darken")]
    #[case("mix-blend-lighten", "mix-blend-mode", "lighten")]
    #[case("mix-blend-color-dodge", "mix-blend-mode", "color-dodge")]
    #[case("mix-blend-color-burn", "mix-blend-mode", "color-burn")]
    #[case("mix-blend-hard-light", "mix-blend-mode", "hard-light")]
    #[case("mix-blend-soft-light", "mix-blend-mode", "soft-light")]
    #[case("mix-blend-difference", "mix-blend-mode", "difference")]
    #[case("mix-blend-exclusion", "mix-blend-mode", "exclusion")]
    #[case("mix-blend-hue", "mix-blend-mode", "hue")]
    #[case("mix-blend-saturation", "mix-blend-mode", "saturation")]
    #[case("mix-blend-color", "mix-blend-mode", "color")]
    #[case("mix-blend-luminosity", "mix-blend-mode", "luminosity")]
    fn test_parse_mix_blend_mode(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse mix-blend");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }

    // Wave 7.10: blur- unknown value (line 2368)
    #[rstest]
    #[case("blur-unknown")]
    #[case("blur-huge")]
    #[case("blur-4xl")]
    fn test_blur_unknown_value_returns_none(#[case] class: &str) {
        let result = parse_single_class(class);
        assert!(result.is_none());
    }

    // Wave 7.11: grayscale-0 (line 2432)
    // Wave 7.12: hue-rotate- unknown value (line 2444)
    // Wave 7.13: saturate- unknown value (line 2465)
    // ============================================================================
    // WAVE 8: Coverage Gap Tests for Lines 816, 866, 295
    // ============================================================================

    // Wave 8.3: parse_tailwind_to_styles integration for rounded variants (via BORDER_RADIUS_SCALE)
    #[test]
    #[serial]
    fn test_parse_tailwind_to_styles_rounded_integration() {
        reset_class_map();
        reset_file_map();

        let styles = parse_tailwind_to_styles(
            "rounded-none rounded-sm rounded-md rounded-lg rounded-xl rounded-2xl rounded-3xl rounded-full",
        );
        assert_eq!(styles.len(), 8);
    }

    // Wave 8.4: parse_tailwind_to_styles integration for border widths (via BORDER_WIDTH_SCALE)
    // ==================== Variants (Tailwind v4) ====================

    #[rstest]
    #[case("first:p-4", "&:first-child")]
    #[case("last:p-4", "&:last-child")]
    #[case("only:p-4", "&:only-child")]
    #[case("odd:p-4", "&:nth-child(odd)")]
    #[case("even:p-4", "&:nth-child(even)")]
    #[case("first-of-type:p-4", "&:first-of-type")]
    #[case("last-of-type:p-4", "&:last-of-type")]
    #[case("only-of-type:p-4", "&:only-of-type")]
    #[case("visited:p-4", "&:visited")]
    #[case("target:p-4", "&:target")]
    #[case("open:p-4", "&:is([open], :popover-open, :open)")]
    #[case("default:p-4", "&:default")]
    #[case("checked:p-4", "&:checked")]
    #[case("indeterminate:p-4", "&:indeterminate")]
    #[case("placeholder-shown:p-4", "&:placeholder-shown")]
    #[case("autofill:p-4", "&:autofill")]
    #[case("optional:p-4", "&:optional")]
    #[case("required:p-4", "&:required")]
    #[case("valid:p-4", "&:valid")]
    #[case("invalid:p-4", "&:invalid")]
    #[case("user-valid:p-4", "&:user-valid")]
    #[case("user-invalid:p-4", "&:user-invalid")]
    #[case("in-range:p-4", "&:in-range")]
    #[case("out-of-range:p-4", "&:out-of-range")]
    #[case("read-only:p-4", "&:read-only")]
    #[case("empty:p-4", "&:empty")]
    #[case("focus-within:p-4", "&:focus-within")]
    #[case("focus:p-4", "&:focus")]
    #[case("focus-visible:p-4", "&:focus-visible")]
    #[case("active:p-4", "&:active")]
    #[case("enabled:p-4", "&:enabled")]
    #[case("disabled:p-4", "&:disabled")]
    #[case("inert:p-4", "&:is([inert], [inert] *)")]
    #[case("placeholder:p-4", "&::placeholder")]
    #[case("file:p-4", "&::file-selector-button")]
    #[case("backdrop:p-4", "&::backdrop")]
    #[case("first-letter:p-4", "&::first-letter")]
    #[case("first-line:p-4", "&::first-line")]
    #[case("details-content:p-4", "&::details-content")]
    #[case("dark:p-4", ":root[data-theme=dark] &")]
    #[case("rtl:p-4", "&:where(:dir(rtl), [dir=\"rtl\"], [dir=\"rtl\"] *)")]
    #[case("ltr:p-4", "&:where(:dir(ltr), [dir=\"ltr\"], [dir=\"ltr\"] *)")]
    #[case("*:p-4", ":is(& > *)")]
    #[case("**:p-4", ":is(& *)")]
    #[case("aria-checked:p-4", "&[aria-checked=\"true\"]")]
    #[case("aria-[sort=ascending]:p-4", "&[aria-sort=\"ascending\"]")]
    #[case("aria-[a=b_c]:p-4", "&[aria-a=\"b c\"]")]
    #[case("aria-[label]:p-4", "&[aria-label]")]
    #[case("data-active:p-4", "&[data-active]")]
    #[case("data-[state=open]:p-4", "&[data-state=\"open\"]")]
    #[case("data-[state='open']:p-4", "&[data-state='open']")]
    #[case("nth-3:p-4", "&:nth-child(3)")]
    #[case("nth-last-3:p-4", "&:nth-last-child(3)")]
    #[case("nth-of-type-3:p-4", "&:nth-of-type(3)")]
    #[case("nth-last-of-type-3:p-4", "&:nth-last-of-type(3)")]
    #[case("nth-[2n+1_of_.x]:p-4", "&:nth-child(2n+1 of .x)")]
    #[case("has-[img]:p-4", "&:has(:is(img))")]
    #[case("has-[.x_y]:p-4", "&:has(:is(.x y))")]
    #[case("has-checked:p-4", "&:has(:checked)")]
    #[case("has-aria-checked:p-4", "&:has([aria-checked=\"true\"])")]
    #[case("group-focus:p-4", "&:is(:where(.group):focus *)")]
    #[case("group-first:p-4", "&:is(:where(.group):first-child *)")]
    #[case("group-data-active:p-4", "&:is(:where(.group)[data-active] *)")]
    #[case("group-has-checked:p-4", "&:is(:where(.group):has(:checked) *)")]
    #[case("group-focus/item:p-4", "&:is(:where(.group\\/item):focus *)")]
    #[case("peer-checked:p-4", "&:is(:where(.peer):checked ~ *)")]
    #[case(
        "peer-aria-checked:p-4",
        "&:is(:where(.peer)[aria-checked=\"true\"] ~ *)"
    )]
    #[case(
        "peer-placeholder-shown:p-4",
        "&:is(:where(.peer):placeholder-shown ~ *)"
    )]
    #[case("[&>*]:p-4", "&>*")]
    #[case("[&_p]:p-4", "& p")]
    #[case("[.dark_&]:p-4", ".dark &")]
    #[case("[&:nth-child(3)]:p-4", "&:nth-child(3)")]
    #[case("hover:focus:p-4", "@media(hover:hover) &:hover:focus")]
    #[case("*:hover:p-4", "@media(hover:hover) :is(& > *):hover")]
    #[case("hover:[&>*]:p-4", "@media(hover:hover) &:hover>*")]
    #[case("[&>*]:hover:p-4", "@media(hover:hover) &>*:hover")]
    #[case(
        "group-hover:focus:p-4",
        "@media(hover:hover) &:is(:where(.group):hover *):focus"
    )]
    #[case("dark:hover:p-4", "@media(hover:hover) :root[data-theme=dark] &:hover")]
    #[case("hover:dark:p-4", "@media(hover:hover) :root[data-theme=dark] &:hover")]
    #[case("before:hover:p-4", "@media(hover:hover) &::before:hover")]
    fn test_selector_variants(#[case] class: &str, #[case] expected: &str) {
        assert_eq!(conditions(class), Some((0, vec![expected.to_string()])));
    }

    #[rstest]
    #[case("hover:p-4", 0, &["@media(hover:hover) &:hover"])]
    #[case("group-hover:p-4", 0, &["@media(hover:hover) &:is(:where(.group):hover *)"])]
    #[case("group-hover/item:p-4", 0, &["@media(hover:hover) &:is(:where(.group\\/item):hover *)"])]
    #[case("peer-hover:p-4", 0, &["@media(hover:hover) &:is(:where(.peer):hover ~ *)"])]
    #[case("motion-safe:p-4", 0, &["@media(prefers-reduced-motion:no-preference)"])]
    #[case("motion-reduce:p-4", 0, &["@media(prefers-reduced-motion:reduce)"])]
    #[case("contrast-more:p-4", 0, &["@media(prefers-contrast:more)"])]
    #[case("contrast-less:p-4", 0, &["@media(prefers-contrast:less)"])]
    #[case("portrait:p-4", 0, &["@media(orientation:portrait)"])]
    #[case("landscape:p-4", 0, &["@media(orientation:landscape)"])]
    #[case("print:p-4", 0, &["@media print"])]
    #[case("forced-colors:p-4", 0, &["@media(forced-colors:active)"])]
    #[case("inverted-colors:p-4", 0, &["@media(inverted-colors:inverted)"])]
    #[case("noscript:p-4", 0, &["@media(scripting:none)"])]
    #[case("pointer-fine:p-4", 0, &["@media(pointer:fine)"])]
    #[case("pointer-coarse:p-4", 0, &["@media(pointer:coarse)"])]
    #[case("pointer-none:p-4", 0, &["@media(pointer:none)"])]
    #[case("any-pointer-fine:p-4", 0, &["@media(any-pointer:fine)"])]
    #[case("any-pointer-coarse:p-4", 0, &["@media(any-pointer:coarse)"])]
    #[case("any-pointer-none:p-4", 0, &["@media(any-pointer:none)"])]
    #[case("supports-[display:grid]:p-4", 0, &["@supports(display:grid)"])]
    #[case("supports-[(display:grid)]:p-4", 0, &["@supports(display:grid)"])]
    #[case("[@media(width>=10px)]:p-4", 0, &["@media(width>=10px)"])]
    #[case("[@supports(display:grid)]:p-4", 0, &["@supports(display:grid)"])]
    #[case("sm:p-4", 1, &[""])]
    #[case("md:p-4", 2, &[""])]
    #[case("lg:p-4", 3, &[""])]
    #[case("xl:p-4", 4, &[""])]
    #[case("2xl:p-4", 5, &[""])]
    #[case("sm:md:p-4", 2, &[""])]
    #[case("md:hover:p-4", 2, &["@media(hover:hover) &:hover"])]
    #[case("hover:md:p-4", 2, &["@media(hover:hover) &:hover"])]
    #[case("selection:p-4", 0, &["& *::selection", "&::selection"])]
    #[case(
        "marker:p-4",
        0,
        &["& *::marker", "&::marker", "& *::-webkit-details-marker", "&::-webkit-details-marker"]
    )]
    #[case("md:selection:p-4", 2, &["& *::selection", "&::selection"])]
    #[case("[&_p,&_li]:p-4", 0, &["& p", "& li"])]
    #[case("[&_p,&_li]:hover:p-4", 0, &["@media(hover:hover) & p:hover", "@media(hover:hover) & li:hover"])]
    fn test_variant_conditions(#[case] class: &str, #[case] level: u8, #[case] expected: &[&str]) {
        assert_eq!(
            conditions(class),
            Some((level, expected.iter().map(ToString::to_string).collect()))
        );
    }

    #[test]
    fn test_stacked_media_variants() {
        let (_, conditions) = conditions("print:hover:p-4").unwrap();
        assert_eq!(conditions.len(), 1);
        assert!(conditions[0].starts_with("@media print"), "{conditions:?}");
        assert!(conditions[0].contains("(hover:hover)"), "{conditions:?}");
        assert!(conditions[0].ends_with("&:hover"), "{conditions:?}");
    }

    // `::before`/`::after` only render with `content`, which Tailwind adds
    #[test]
    fn test_pseudo_element_content() {
        assert_eq!(
            declarations("before:block"),
            Some(owned(&[
                ("content", "var(--tw-content)"),
                ("display", "block")
            ]))
        );
        assert_eq!(
            declarations("after:content-['x']"),
            Some(owned(&[
                ("--tw-content", "'x'"),
                ("content", "var(--tw-content)")
            ]))
        );
        assert_eq!(
            declarations("placeholder:block"),
            Some(owned(&[("display", "block")]))
        );
    }

    // A class whose variants or utility are not understood in full stays as written
    #[rstest]
    #[case("custom")]
    #[case("prose")]
    #[case("my-p-4-class")]
    #[case("analytics-hook")]
    #[case("container")]
    #[case("group")]
    #[case("peer")]
    #[case("group/item")]
    #[case("")]
    #[case(":p-4")]
    #[case("hover:")]
    #[case("hover::p-4")]
    #[case("not-hover:p-4")]
    #[case("in-focus:p-4")]
    #[case("max-md:p-4")]
    #[case("min-[700px]:p-4")]
    #[case("@container:p-4")]
    #[case("starting:p-4")]
    #[case("screen:p-4")]
    #[case("unknown:p-4")]
    #[case("[.x]:p-4")]
    #[case("[@foo_x]:p-4")]
    #[case("[&]]:p-4")]
    #[case("supports-grid:p-4")]
    #[case("supports-[grid]:p-4")]
    #[case("group-[.x]:p-4")]
    #[case("group-unknown:p-4")]
    #[case("group-hover/a.b:p-4")]
    #[case("peer-hover/:p-4")]
    #[case("has-hover:p-4")]
    #[case("has-has-[x]:p-4")]
    #[case("has-[&>p]:p-4")]
    #[case("has-unknown:p-4")]
    #[case("aria-foo:p-4")]
    #[case("aria-[a~=b]:p-4")]
    #[case("data-[]:p-4")]
    #[case("data-[a=]:p-4")]
    #[case("data-[a=b\"c]:p-4")]
    #[case("data-[a_b]:p-4")]
    #[case("data-a.b:p-4")]
    #[case("nth-0:p-4")]
    #[case("nth-x:p-4")]
    #[case("print:[@media_screen]:p-4")]
    #[case("text-sm/x")]
    #[case("text-sm/[_]")]
    #[case("text-sm/(x)")]
    #[case("select-wrapper")]
    #[case("order-summary")]
    #[case("cursor-foo")]
    #[case("align-left")]
    #[case("whitespace-x")]
    #[case("hyphens-x")]
    #[case("pointer-events-x")]
    #[case("touch-x")]
    #[case("mix-blend-x")]
    #[case("bg-blend-plus-lighter")]
    #[case("col-start-x")]
    #[case("col-end-x")]
    #[case("row-start-x")]
    #[case("row-end-x")]
    #[case("columns-x")]
    #[case("aspect-x")]
    #[case("aspect-4-3")]
    #[case("aspect-a/3")]
    #[case("leading-custom")]
    #[case("leading-(x)")]
    #[case("-p-4")]
    #[case("-m-auto")]
    #[case("-order-first")]
    #[case("-size-4")]
    #[case("-truncate")]
    #[case("-[color:red]")]
    #[case("-p-[4px]")]
    #[case("--[1px]")]
    #[case("[Color:red]")]
    #[case("[--x.y:1]")]
    #[case("[color:_]")]
    #[case("[colorred]")]
    #[case("[color:red")]
    #[case("w-[]")]
    #[case("w-]")]
    #[case("w[1px]")]
    #[case("foo-[1px]")]
    #[case("w-(x)")]
    #[case("w-[_]")]
    #[case("bg-[size:cover]")]
    #[case("text-[2]")]
    #[case("text-[angle:3deg]")]
    #[case("bg-[10px]")]
    #[case("border-[3]")]
    #[case("border-[url(x)]")]
    #[case("borderx-[3px]")]
    #[case("border-z-[3px]")]
    #[case("roundedx-[3px]")]
    #[case("rounded-z-[3px]")]
    #[case("rounded-huge")]
    #[case("roundedx")]
    #[case("font-[2px]")]
    #[case("outline-[url(x)]")]
    #[case("size-screen")]
    #[case("translate-x-auto")]
    #[case("translate-x-screen")]
    #[case("translate-x")]
    #[case("scale-1.5")]
    #[case("skew-x")]
    fn test_preserved_classes(#[case] class: &str) {
        assert_eq!(parse_class(class), None, "{class}");
    }

    // ==================== Utilities with several declarations ====================

    #[rstest]
    #[case("text-xs", "0.75rem", "var(--tw-leading, calc(1 / 0.75))")]
    #[case("text-sm", "0.875rem", "var(--tw-leading, calc(1.25 / 0.875))")]
    #[case("text-base", "1rem", "var(--tw-leading, calc(1.5 / 1))")]
    #[case("text-xl", "1.25rem", "var(--tw-leading, calc(1.75 / 1.25))")]
    #[case("text-9xl", "8rem", "var(--tw-leading, 1)")]
    #[case("text-sm/6", "0.875rem", "1.5rem")]
    #[case("text-sm/[1.7]", "0.875rem", "1.7")]
    #[case("text-sm/tight", "0.875rem", "1.25")]
    #[case("text-sm/(--lh)", "0.875rem", "var(--lh)")]
    fn test_font_size_utilities(
        #[case] class: &str,
        #[case] font_size: &str,
        #[case] line_height: &str,
    ) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                ("font-size", font_size),
                ("line-height", line_height)
            ]))
        );
    }

    #[test]
    fn test_screen_reader_utilities() {
        assert_eq!(
            declarations("sr-only"),
            Some(owned(&[
                ("position", "absolute"),
                ("width", "1px"),
                ("height", "1px"),
                ("padding", "0"),
                ("margin", "-1px"),
                ("overflow", "hidden"),
                ("clip-path", "inset(50%)"),
                ("white-space", "nowrap"),
                ("border-width", "0"),
            ]))
        );
        assert_eq!(
            declarations("not-sr-only"),
            Some(owned(&[
                ("position", "static"),
                ("width", "auto"),
                ("height", "auto"),
                ("padding", "0"),
                ("margin", "0"),
                ("overflow", "visible"),
                ("clip-path", "none"),
                ("white-space", "normal"),
            ]))
        );
    }

    #[rstest]
    #[case("-m-4", "margin", "-1rem")]
    #[case("-mt-4", "margin-top", "-1rem")]
    #[case("-inset-x-4", "inset-inline", "-1rem")]
    #[case("-top-2", "top", "-0.5rem")]
    #[case("-z-10", "z-index", "-10")]
    #[case("-order-1", "order", "-1")]
    #[case("-tracking-wide", "letter-spacing", "-0.025em")]
    #[case("-scroll-m-4", "scroll-margin", "-1rem")]
    #[case("-m-[4px]", "margin", "calc(4px * -1)")]
    #[case("-top-[3px]", "top", "calc(3px * -1)")]
    #[case("-z-[3]", "z-index", "calc(3 * -1)")]
    #[case("-rotate-[30deg]", "rotate", "calc(30deg * -1)")]
    #[case("mt-[-3px]", "margin-top", "-3px")]
    fn test_negative_utilities(#[case] class: &str, #[case] property: &str, #[case] value: &str) {
        assert_eq!(declarations(class), Some(owned(&[(property, value)])));
    }

    // ==================== Arbitrary values ====================

    #[rstest]
    #[case("[mask-type:luminance]", "mask-type", "luminance")]
    #[case("[--my_var:1px_2px]", "--my_var", "1px 2px")]
    #[case("[margin:calc(1px+2px)]", "margin", "calc(1px + 2px)")]
    #[case("[-webkit-line-clamp:3]", "-webkit-line-clamp", "3")]
    #[case("bg-(--my-color)", "background-color", "var(--my-color)")]
    #[case("bg-(image:--x)", "background-image", "var(--x)")]
    #[case("text-(length:--my-size)", "font-size", "var(--my-size)")]
    #[case("text-(--x)", "color", "var(--x)")]
    #[case("w-(--x,10px)", "width", "var(--x,10px)")]
    #[case("p-(length:--x)", "padding", "var(--x)")]
    #[case("text-[2rem]", "font-size", "2rem")]
    #[case("text-[1.5em]", "font-size", "1.5em")]
    #[case("text-[large]", "font-size", "large")]
    #[case("text-[calc(1rem+2px)]", "font-size", "calc(1rem + 2px)")]
    #[case("text-[length:var(--x)]", "font-size", "var(--x)")]
    #[case("text-[percentage:50%]", "font-size", "50%")]
    #[case("text-[absolute-size:large]", "font-size", "large")]
    #[case("text-[#fff]", "color", "#fff")]
    #[case("text-[var(--x)]", "color", "var(--x)")]
    #[case("text-[color:var(--x)]", "color", "var(--x)")]
    #[case("bg-[url(/a_b.png)]", "background-image", "url(/a_b.png)")]
    #[case("bg-[url('/a_b.png')]", "background-image", "url('/a_b.png')")]
    #[case(
        "bg-[linear-gradient(red,blue)]",
        "background-image",
        "linear-gradient(red,blue)"
    )]
    #[case("bg-[image:var(--x)]", "background-image", "var(--x)")]
    #[case("bg-[url:var(--x)]", "background-image", "var(--x)")]
    #[case("bg-[#123456]", "background-color", "#123456")]
    #[case("bg-[var(--x)]", "background-color", "var(--x)")]
    #[case("bg-[length:200px_100px]", "background-size", "200px 100px")]
    #[case("bg-[bg-size:cover]", "background-size", "cover")]
    #[case("bg-[position:top]", "background-position", "top")]
    #[case("border-[thin]", "border-width", "thin")]
    #[case("border-[length:var(--x)]", "border-width", "var(--x)")]
    #[case("border-[line-width:var(--x)]", "border-width", "var(--x)")]
    #[case("border-[#fff]", "border-color", "#fff")]
    #[case("border-[var(--x)]", "border-color", "var(--x)")]
    #[case("border-t-[red]", "border-top-color", "red")]
    #[case("border-s-[red]", "border-inline-start-color", "red")]
    #[case("outline-[red]", "outline-color", "red")]
    #[case("stroke-[3px]", "stroke-width", "3px")]
    #[case("stroke-[2]", "stroke-width", "2")]
    #[case("stroke-[red]", "stroke", "red")]
    #[case("decoration-[3px]", "text-decoration-thickness", "3px")]
    #[case("decoration-[red]", "text-decoration-color", "red")]
    #[case("font-[600]", "font-weight", "600")]
    #[case("font-[number:var(--w)]", "font-weight", "var(--w)")]
    #[case("font-[Inter_Var]", "font-family", "Inter Var")]
    #[case("font-[family-name:var(--f)]", "font-family", "var(--f)")]
    #[case("w-[calc(100%_-_2rem)]", "width", "calc(100% - 2rem)")]
    #[case("m-[var(--x_y)]", "margin", "var(--x_y)")]
    #[case(
        "grid-cols-[repeat(2,minmax(0,1fr))]",
        "grid-template-columns",
        "repeat(2,minmax(0,1fr))"
    )]
    #[case("col-[1/3]", "grid-column", "1/3")]
    #[case("row-[span_2]", "grid-row", "span 2")]
    #[case("origin-[33%_75%]", "transform-origin", "33% 75%")]
    #[case("object-[25%_75%]", "object-position", "25% 75%")]
    fn test_arbitrary_utilities(#[case] class: &str, #[case] property: &str, #[case] value: &str) {
        assert_eq!(declarations(class), Some(owned(&[(property, value)])));
    }

    #[rstest]
    #[case("content-['a_b']", "'a b'")]
    #[case("content-['hello\\_world']", "'hello_world'")]
    #[case("content-[attr(data-a_b)]", "attr(data-a b)")]
    fn test_arbitrary_content(#[case] class: &str, #[case] value: &str) {
        assert_eq!(
            declarations(class),
            Some(owned(&[
                ("--tw-content", value),
                ("content", "var(--tw-content)")
            ]))
        );
    }

    // Values decoded like Tailwind does, checked against Tailwind CSS 4.3.3
    #[rstest]
    #[case("calc(100%-2rem)", "calc(100% - 2rem)")]
    #[case("calc(1rem+2px)", "calc(1rem + 2px)")]
    #[case("calc(2*3px)", "calc(2 * 3px)")]
    #[case("calc(10px/2)", "calc(10px / 2)")]
    #[case("min(10px,2rem)", "min(10px, 2rem)")]
    #[case("max(10px,calc(1rem-2px))", "max(10px, calc(1rem - 2px))")]
    #[case("clamp(1rem,2vw+1px,3rem)", "clamp(1rem, 2vw + 1px, 3rem)")]
    #[case("calc(100%-var(--x))", "calc(100% - var(--x))")]
    #[case("calc(var(--a)*-1)", "calc(var(--a) * -1)")]
    #[case("calc(-1*2px)", "calc(-1 * 2px)")]
    #[case("calc(1e-3px+1px)", "calc(1e-3px + 1px)")]
    #[case("calc(100%_-_2rem)", "calc(100% - 2rem)")]
    #[case("calc(1px_+2px)", "calc(1px + 2px)")]
    #[case("calc((1px+2px)*3)", "calc((1px + 2px) * 3)")]
    #[case("calc(1px--2px)", "calc(1px - -2px)")]
    #[case("calc(a-b)", "calc(a-b)")]
    #[case("var(--a_b,calc(1px+2px))", "var(--a_b,calc(1px + 2px))")]
    #[case("var(calc(1px+2px))", "var(calc(1px + 2px))")]
    #[case("theme(spacing_4)", "theme(spacing_4)")]
    #[case("x_var(--a_b)", "x var(--a_b)")]
    #[case("x_theme(--a_b)", "x theme(--a_b)")]
    #[case("x_url(a_b)", "x url(a_b)")]
    #[case("repeat(2,minmax(0,1fr))", "repeat(2,minmax(0,1fr))")]
    #[case("url(/a_b.png)", "url(/a_b.png)")]
    #[case("url(a_b", "url(a_b")]
    #[case("attr(data-a_b)", "attr(data-a b)")]
    #[case("a_(b)_c", "a (b) c")]
    #[case("a)b_c", "a)b c")]
    #[case("a)b_(c)", "a)b (c)")]
    #[case("attr(a\\_b)", "attr(a_b)")]
    #[case("url(a(b)_c)", "url(a(b)_c)")]
    #[case("'a_b'", "'a b'")]
    #[case("'hello\\_world'", "'hello_world'")]
    #[case("한_글", "한 글")]
    fn test_decode_arbitrary_value(#[case] value: &str, #[case] expected: &str) {
        assert_eq!(decode_arbitrary_value(value), expected);
    }

    #[test]
    fn test_balanced_brackets() {
        assert!(is_balanced("[a](b)"));
        assert!(!is_balanced("(]"));
        assert!(!is_balanced("[)"));
        assert!(!is_balanced("("));
    }

    #[rstest]
    #[case("inset-y-4", "inset-block", "1rem")]
    #[case("inset-x-2", "inset-inline", "0.5rem")]
    #[case("basis-4", "flex-basis", "1rem")]
    #[case("basis-13", "flex-basis", "3.25rem")]
    #[case("top-13", "top", "3.25rem")]
    fn test_parse_spacing_scale_steps(
        #[case] class: &str,
        #[case] expected_prop: &str,
        #[case] expected_value: &str,
    ) {
        let parsed = parse_single_class(class).expect("Should parse");
        assert_eq!(parsed.property, expected_prop);
        assert_eq!(parsed.value, expected_value);
    }
    #[test]
    fn test_styles_and_properties() {
        let class = parse_class("md:hover:p-4").unwrap();
        let styles: Vec<ExtractStaticStyle> = class.styles().collect();
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].property(), "padding");
        assert_eq!(styles[0].value(), "1rem");
        assert_eq!(styles[0].level(), 2);
        assert_eq!(
            styles[0].selector().map(ToString::to_string).as_deref(),
            Some("@media(hover:hover) &:hover")
        );
        assert_eq!(class.rules(), Vec::<&str>::new());

        let class = parse_class("selection:translate-x-4").unwrap();
        assert_eq!(class.styles().count(), 4);
        assert_eq!(
            class.rules(),
            vec![
                "@property --tw-translate-x{syntax:\"*\";inherits:false;initial-value:0}",
                "@property --tw-translate-y{syntax:\"*\";inherits:false;initial-value:0}",
            ]
        );
        assert_eq!(
            parse_class("before:block").unwrap().rules(),
            vec!["@property --tw-content{syntax:\"*\";inherits:false;initial-value:\"\"}"]
        );
    }
}
