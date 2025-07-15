use phf::phf_set;

pub(super) static MAINTAIN_VALUE_PROPERTIES: phf::Set<&str> = phf_set! {
    "opacity",
    "flex",
    "zIndex",
    "lineClamp",
    "fontWeight",
    "lineHeight",
    "scale",
    "aspectRatio",
    "flexGrow",
    "flexShrink",
    "order",
    "gridColumn",
    "gridColumnStart",
    "gridColumnEnd",
    "gridRow",
    "gridRowStart",
    "gridRowEnd",
    "animationIterationCount"
};
