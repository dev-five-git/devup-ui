//! The order atomic rules take so a longhand always follows the shorthands that
//! cover it.
//!
//! Atomic rules of equal specificity win by their place in the sheet. Sorting them
//! by name alone put `border-width` after `border-top-width` and `place-items` after
//! `align-items`, so the shorthand won. A property is sorted under the greatest name
//! among itself and every shorthand covering it, then by how deeply it is covered,
//! so a covered property comes right after its last shorthand and every other pair
//! keeps its alphabetical order.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Each CSS shorthand and the properties it sets directly.
static SHORTHANDS: &[(&str, &[&str])] = &[
    (
        "animation",
        &[
            "animation-name",
            "animation-duration",
            "animation-timing-function",
            "animation-delay",
            "animation-iteration-count",
            "animation-direction",
            "animation-fill-mode",
            "animation-play-state",
            "animation-timeline",
            "animation-range",
        ],
    ),
    (
        "animation-range",
        &["animation-range-start", "animation-range-end"],
    ),
    (
        "background",
        &[
            "background-attachment",
            "background-clip",
            "background-color",
            "background-image",
            "background-origin",
            "background-position",
            "background-repeat",
            "background-size",
        ],
    ),
    (
        "background-position",
        &["background-position-x", "background-position-y"],
    ),
    (
        "border",
        &[
            "border-color",
            "border-style",
            "border-width",
            "border-top",
            "border-right",
            "border-bottom",
            "border-left",
            "border-image",
        ],
    ),
    (
        "border-top",
        &["border-top-color", "border-top-style", "border-top-width"],
    ),
    (
        "border-right",
        &[
            "border-right-color",
            "border-right-style",
            "border-right-width",
        ],
    ),
    (
        "border-bottom",
        &[
            "border-bottom-color",
            "border-bottom-style",
            "border-bottom-width",
        ],
    ),
    (
        "border-left",
        &[
            "border-left-color",
            "border-left-style",
            "border-left-width",
        ],
    ),
    (
        "border-color",
        &[
            "border-top-color",
            "border-right-color",
            "border-bottom-color",
            "border-left-color",
        ],
    ),
    (
        "border-style",
        &[
            "border-top-style",
            "border-right-style",
            "border-bottom-style",
            "border-left-style",
        ],
    ),
    (
        "border-width",
        &[
            "border-top-width",
            "border-right-width",
            "border-bottom-width",
            "border-left-width",
        ],
    ),
    (
        "border-block",
        &[
            "border-block-color",
            "border-block-style",
            "border-block-width",
            "border-block-start",
            "border-block-end",
        ],
    ),
    (
        "border-block-start",
        &[
            "border-block-start-color",
            "border-block-start-style",
            "border-block-start-width",
        ],
    ),
    (
        "border-block-end",
        &[
            "border-block-end-color",
            "border-block-end-style",
            "border-block-end-width",
        ],
    ),
    (
        "border-block-color",
        &["border-block-start-color", "border-block-end-color"],
    ),
    (
        "border-block-style",
        &["border-block-start-style", "border-block-end-style"],
    ),
    (
        "border-block-width",
        &["border-block-start-width", "border-block-end-width"],
    ),
    (
        "border-inline",
        &[
            "border-inline-color",
            "border-inline-style",
            "border-inline-width",
            "border-inline-start",
            "border-inline-end",
        ],
    ),
    (
        "border-inline-start",
        &[
            "border-inline-start-color",
            "border-inline-start-style",
            "border-inline-start-width",
        ],
    ),
    (
        "border-inline-end",
        &[
            "border-inline-end-color",
            "border-inline-end-style",
            "border-inline-end-width",
        ],
    ),
    (
        "border-inline-color",
        &["border-inline-start-color", "border-inline-end-color"],
    ),
    (
        "border-inline-style",
        &["border-inline-start-style", "border-inline-end-style"],
    ),
    (
        "border-inline-width",
        &["border-inline-start-width", "border-inline-end-width"],
    ),
    (
        "border-image",
        &[
            "border-image-outset",
            "border-image-repeat",
            "border-image-slice",
            "border-image-source",
            "border-image-width",
        ],
    ),
    (
        "border-radius",
        &[
            "border-top-left-radius",
            "border-top-right-radius",
            "border-bottom-right-radius",
            "border-bottom-left-radius",
        ],
    ),
    (
        "column-rule",
        &[
            "column-rule-color",
            "column-rule-style",
            "column-rule-width",
        ],
    ),
    ("columns", &["column-count", "column-width"]),
    (
        "contain-intrinsic-size",
        &["contain-intrinsic-width", "contain-intrinsic-height"],
    ),
    ("container", &["container-name", "container-type"]),
    ("flex", &["flex-basis", "flex-grow", "flex-shrink"]),
    ("flex-flow", &["flex-direction", "flex-wrap"]),
    (
        "font",
        &[
            "font-family",
            "font-size",
            "font-stretch",
            "font-style",
            "font-variant",
            "font-weight",
            "line-height",
            "font-size-adjust",
            "font-kerning",
            "font-optical-sizing",
            "font-variation-settings",
            "font-feature-settings",
            "font-language-override",
        ],
    ),
    (
        "font-variant",
        &[
            "font-variant-alternates",
            "font-variant-caps",
            "font-variant-east-asian",
            "font-variant-emoji",
            "font-variant-ligatures",
            "font-variant-numeric",
            "font-variant-position",
        ],
    ),
    (
        "font-synthesis",
        &[
            "font-synthesis-position",
            "font-synthesis-small-caps",
            "font-synthesis-style",
            "font-synthesis-weight",
        ],
    ),
    ("gap", &["row-gap", "column-gap"]),
    ("grid-gap", &["grid-row-gap", "grid-column-gap"]),
    (
        "grid",
        &[
            "grid-auto-columns",
            "grid-auto-flow",
            "grid-auto-rows",
            "grid-template",
        ],
    ),
    (
        "grid-template",
        &[
            "grid-template-areas",
            "grid-template-columns",
            "grid-template-rows",
        ],
    ),
    ("grid-area", &["grid-row", "grid-column"]),
    ("grid-row", &["grid-row-start", "grid-row-end"]),
    ("grid-column", &["grid-column-start", "grid-column-end"]),
    ("inset", &["top", "right", "bottom", "left"]),
    ("inset-block", &["inset-block-start", "inset-block-end"]),
    ("inset-inline", &["inset-inline-start", "inset-inline-end"]),
    (
        "list-style",
        &["list-style-image", "list-style-position", "list-style-type"],
    ),
    (
        "margin",
        &["margin-top", "margin-right", "margin-bottom", "margin-left"],
    ),
    ("margin-block", &["margin-block-start", "margin-block-end"]),
    (
        "margin-inline",
        &["margin-inline-start", "margin-inline-end"],
    ),
    (
        "mask",
        &[
            "mask-clip",
            "mask-composite",
            "mask-image",
            "mask-mode",
            "mask-origin",
            "mask-position",
            "mask-repeat",
            "mask-size",
            "mask-border",
        ],
    ),
    (
        "mask-border",
        &[
            "mask-border-mode",
            "mask-border-outset",
            "mask-border-repeat",
            "mask-border-slice",
            "mask-border-source",
            "mask-border-width",
        ],
    ),
    (
        "offset",
        &[
            "offset-anchor",
            "offset-distance",
            "offset-path",
            "offset-position",
            "offset-rotate",
        ],
    ),
    (
        "outline",
        &["outline-color", "outline-style", "outline-width"],
    ),
    ("overflow", &["overflow-x", "overflow-y"]),
    (
        "overscroll-behavior",
        &["overscroll-behavior-x", "overscroll-behavior-y"],
    ),
    (
        "padding",
        &[
            "padding-top",
            "padding-right",
            "padding-bottom",
            "padding-left",
        ],
    ),
    (
        "padding-block",
        &["padding-block-start", "padding-block-end"],
    ),
    (
        "padding-inline",
        &["padding-inline-start", "padding-inline-end"],
    ),
    ("place-content", &["align-content", "justify-content"]),
    ("place-items", &["align-items", "justify-items"]),
    ("place-self", &["align-self", "justify-self"]),
    (
        "position-try",
        &["position-try-order", "position-try-fallbacks"],
    ),
    (
        "scroll-margin",
        &[
            "scroll-margin-top",
            "scroll-margin-right",
            "scroll-margin-bottom",
            "scroll-margin-left",
        ],
    ),
    (
        "scroll-margin-block",
        &["scroll-margin-block-start", "scroll-margin-block-end"],
    ),
    (
        "scroll-margin-inline",
        &["scroll-margin-inline-start", "scroll-margin-inline-end"],
    ),
    (
        "scroll-padding",
        &[
            "scroll-padding-top",
            "scroll-padding-right",
            "scroll-padding-bottom",
            "scroll-padding-left",
        ],
    ),
    (
        "scroll-padding-block",
        &["scroll-padding-block-start", "scroll-padding-block-end"],
    ),
    (
        "scroll-padding-inline",
        &["scroll-padding-inline-start", "scroll-padding-inline-end"],
    ),
    (
        "scroll-timeline",
        &["scroll-timeline-name", "scroll-timeline-axis"],
    ),
    (
        "view-timeline",
        &[
            "view-timeline-name",
            "view-timeline-axis",
            "view-timeline-inset",
        ],
    ),
    (
        "text-decoration",
        &[
            "text-decoration-color",
            "text-decoration-line",
            "text-decoration-style",
            "text-decoration-thickness",
        ],
    ),
    (
        "text-emphasis",
        &["text-emphasis-color", "text-emphasis-style"],
    ),
    ("text-wrap", &["text-wrap-mode", "text-wrap-style"]),
    ("white-space", &["white-space-collapse", "text-wrap-mode"]),
    (
        "transition",
        &[
            "transition-behavior",
            "transition-delay",
            "transition-duration",
            "transition-property",
            "transition-timing-function",
        ],
    ),
    (
        "-webkit-mask",
        &[
            "-webkit-mask-attachment",
            "-webkit-mask-clip",
            "-webkit-mask-composite",
            "-webkit-mask-image",
            "-webkit-mask-origin",
            "-webkit-mask-position",
            "-webkit-mask-repeat",
            "-webkit-mask-size",
        ],
    ),
    (
        "-webkit-mask-position",
        &["-webkit-mask-position-x", "-webkit-mask-position-y"],
    ),
    (
        "-webkit-mask-repeat",
        &["-webkit-mask-repeat-x", "-webkit-mask-repeat-y"],
    ),
    (
        "-webkit-text-stroke",
        &["-webkit-text-stroke-color", "-webkit-text-stroke-width"],
    ),
];

/// The sort key of every property set through a shorthand:
/// (greatest name among it and its shorthands, how deeply it is covered).
static COVERED: LazyLock<HashMap<&'static str, (&'static str, u8)>> = LazyLock::new(|| {
    let mut parents: HashMap<&'static str, Vec<&'static str>> = HashMap::new();
    for (shorthand, longhands) in SHORTHANDS {
        for longhand in *longhands {
            parents.entry(longhand).or_default().push(shorthand);
        }
    }
    let mut keys = HashMap::new();
    for property in parents.keys() {
        covered_key(property, &parents, &mut keys);
    }
    keys
});

fn covered_key(
    property: &'static str,
    parents: &HashMap<&'static str, Vec<&'static str>>,
    keys: &mut HashMap<&'static str, (&'static str, u8)>,
) -> (&'static str, u8) {
    if let Some(key) = keys.get(property) {
        return *key;
    }
    let key = parents.get(property).map_or((property, 0), |shorthands| {
        shorthands
            .iter()
            .fold((property, 0), |(anchor, rank), shorthand| {
                let (parent_anchor, parent_rank) = covered_key(shorthand, parents, keys);
                (anchor.max(parent_anchor), rank.max(parent_rank + 1))
            })
    });
    keys.insert(property, key);
    key
}

/// Where `property` sorts: `all` resets every other property, so it sorts first.
fn property_key(property: &str) -> (&str, u8) {
    if property == "all" {
        return ("", 0);
    }
    COVERED.get(property).copied().unwrap_or((property, 0))
}

/// Order two kebab-case properties so a shorthand comes before every property it
/// covers, keeping the alphabetical order of unrelated properties.
pub fn cmp_properties(a: &str, b: &str) -> Ordering {
    property_key(a).cmp(&property_key(b)).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn every_shorthand_sorts_before_what_it_covers() {
        fn covers(shorthand: &str, property: &str) -> bool {
            SHORTHANDS.iter().any(|(name, longhands)| {
                *name == shorthand
                    && longhands
                        .iter()
                        .any(|longhand| *longhand == property || covers(longhand, property))
            })
        }
        let names: Vec<&str> = SHORTHANDS
            .iter()
            .flat_map(|(name, longhands)| std::iter::once(*name).chain(longhands.iter().copied()))
            .collect();
        for shorthand in &names {
            for property in &names {
                if covers(shorthand, property) {
                    assert_eq!(
                        cmp_properties(shorthand, property),
                        Ordering::Less,
                        "{shorthand} must sort before {property}"
                    );
                }
            }
        }
    }

    #[rstest]
    #[case("border-width", "border-top-width", Ordering::Less)]
    #[case("place-items", "align-items", Ordering::Less)]
    #[case("border", "border-top-width", Ordering::Less)]
    #[case("white-space", "text-wrap-mode", Ordering::Less)]
    #[case("padding", "padding-top", Ordering::Less)]
    #[case("all", "accent-color", Ordering::Less)]
    #[case("accent-color", "all", Ordering::Greater)]
    #[case("color", "display", Ordering::Less)]
    #[case("--x", "color", Ordering::Less)]
    #[case("margin-top", "margin-top", Ordering::Equal)]
    #[case("align-items", "display", Ordering::Greater)]
    fn orders_properties(#[case] a: &str, #[case] b: &str, #[case] expected: Ordering) {
        assert_eq!(cmp_properties(a, b), expected);
    }
}
