use phf::phf_set;

/// Properties whose numbers are unitless in CSS, so they are kept as written
/// instead of being scaled to `px`. Includes every property vanilla-extract
/// leaves unitless, so numbers from `.css.ts` files keep their meaning,
/// `content`, which never takes a length, and the counters.
pub(crate) static MAINTAIN_VALUE_PROPERTIES: phf::Set<&str> = phf_set! {
    "content",
    "counter-increment",
    "counter-reset",
    "counter-set",
    "font-size-adjust",
    "math-depth",
    "opacity",
    "flex",
    "z-index",
    "line-clamp",
    "font-weight",
    "line-height",
    "scale",
    "aspect-ratio",
    "flex-grow",
    "flex-shrink",
    "flex-order",
    "flex-positive",
    "flex-negative",
    "order",
    "grid-area",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "grid-column-span",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "grid-row-span",
    "animation-iteration-count",
    "tab-size",
    "border-image",
    "border-image-outset",
    "border-image-slice",
    "border-image-width",
    "box-flex",
    "box-flex-group",
    "box-ordinal-group",
    "column-count",
    "columns",
    "initial-letter",
    "max-lines",
    "orphans",
    "widows",
    "zoom",
    "fill-opacity",
    "flood-opacity",
    "mask-border",
    "mask-border-outset",
    "mask-border-slice",
    "mask-border-width",
    "shape-image-threshold",
    "stop-opacity",
    "stroke-dashoffset",
    "stroke-miterlimit",
    "stroke-opacity",
    "stroke-width"
};

/// Strip one known vendor prefix for classification, preserving unknown names.
pub(crate) fn strip_vendor_prefix(property: &str) -> &str {
    let prefixed = property.strip_prefix('-').unwrap_or(property);
    ["webkit-", "moz-", "ms-", "o-"]
        .into_iter()
        .find_map(|prefix| prefixed.strip_prefix(prefix))
        .unwrap_or(property)
}

/// Classify numeric values independently of vendor spelling, without changing
/// the property name emitted in CSS or deciding whether that name is valid.
pub(crate) fn is_maintain_value_property(property: &str) -> bool {
    MAINTAIN_VALUE_PROPERTIES.contains(strip_vendor_prefix(property))
}

/// Properties taking a time, whose numbers are milliseconds
pub(crate) static TIME_PROPERTIES: phf::Set<&str> = phf_set! {
    "transition-duration",
    "transition-delay",
    "animation-duration",
    "animation-delay",
};
