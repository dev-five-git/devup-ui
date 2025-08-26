use phf::phf_set;

pub(super) static MAINTAIN_VALUE_PROPERTIES: phf::Set<&str> = phf_set! {
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
    "order",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "animation-iteration-count",
    "tab-size",
    "moz-tab-size",
    "webkit-line-clamp"
};
