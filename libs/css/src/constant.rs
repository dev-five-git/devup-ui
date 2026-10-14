use crate::utils::compile_regex;
use phf::{phf_map, phf_set};
use regex_lite::Regex;
use std::sync::LazyLock;

pub(super) const SELECTOR_ORDER: [(&str, u8); 6] = [
    (":hover", 0),
    (":focus-visible", 1),
    (":focus", 2),
    (":active", 3),
    (":selected", 4),
    (":disabled", 5),
];

pub(super) static GLOBAL_STYLE_PROPERTY: phf::Map<&str, &[&str]> = phf_map! {
    "bg" => &["background"],
    "bgAttachment" => &["background-attachment"],
    "bgClip" => &["background-clip"],
    "bgColor" => &["background-color"],
    "bgImage" => &["background-image"],
    "bgImg" => &["background-image"],
    "bgOrigin" => &["background-origin"],
    "bgPosition" => &["background-position"],
    "bgPositionX" => &["background-position-x"],
    "bgPositionY" => &["background-position-y"],
    "bgPos" => &["background-position"],
    "bgPosX" => &["background-position-x"],
    "bgPosY" => &["background-position-y"],
    "bgRepeat" => &["background-repeat"],
    "bgSize" => &["background-size"],
    "bgBlendMode" => &["background-blend-mode"],
    "backgroundImg" => &["background-image"],
    "animationDir" => &["animation-direction"],
    "flexDir" => &["flex-direction"],
    "pos" => &["position"],
    "m" => &["margin"],
    "mt" => &["margin-top"],
    "mr" => &["margin-right"],
    "mb" => &["margin-bottom"],
    "ml" => &["margin-left"],
    "p" => &["padding"],
    "pt" => &["padding-top"],
    "pr" => &["padding-right"],
    "pb" => &["padding-bottom"],
    "pl" => &["padding-left"],
    "w" => &["width"],
    "h" => &["height"],
    "minW" => &["min-width"],
    "minH" => &["min-height"],
    "maxW" => &["max-width"],
    "maxH" => &["max-height"],
    "mx" => &["margin-left", "margin-right"],
    "my" => &["margin-top", "margin-bottom"],
    "px" => &["padding-left", "padding-right"],
    "py" => &["padding-top", "padding-bottom"],
    "boxSize" => &["width", "height"],
    "borderBottomRadius" => &["border-bottom-left-radius", "border-bottom-right-radius"],
    "borderTopRadius" => &["border-top-left-radius", "border-top-right-radius"],
    "borderLeftRadius" => &["border-top-left-radius", "border-bottom-left-radius"],
    "borderRightRadius" => &["border-top-right-radius", "border-bottom-right-radius"],
    "objectPos" => &["object-position"],
    "offsetPos" => &["offset-position"],
    "maskPos" => &["mask-position"],
    "maskImg" => &["mask-image"],
};

pub(super) static GLOBAL_ENUM_STYLE_PROPERTY: phf::Map<&str, phf::Map<&str, phf::Map<&str, &str>>> = phf_map! {
    "positioning" => phf_map!  {
        "top" => phf_map! {
            "top" => "0",
        },
        "right" => phf_map! {
            "right" => "0",
        },
        "bottom" => phf_map! {
            "bottom" => "0",
        },
        "left" => phf_map! {
            "left" => "0",
        },
        "top-right" => phf_map! {
            "top" => "0",
            "right" => "0",
        },
        "top-left" => phf_map! {
            "top" => "0",
            "left" => "0",
        },
        "bottom-left" => phf_map! {
            "bottom" => "0",
            "left" => "0",
        },
        "bottom-right" => phf_map! {
            "bottom" => "0",
            "right" => "0",
        },
    }
};
pub(super) static OPTIMIZE_MULTI_CSS_VALUE_PROPERTY: phf::Set<&str> = phf_set! {
    "font-family",
    "src",
    "animation-name",
};

pub(super) static DOUBLE_SEPARATOR: phf::Set<&str> = phf_set! {
    "after",
    "backdrop",
    "before",
    "checkmark",
    "cue",
    "cue-region",
    "details-content",
    "file-selector-button",
    "first-letter",
    "first-line",
    "grammar-error",
    "highlight",
    "marker",
    "part",
    "picker",
    "picker-icon",
    "placeholder",
    "scroll-marker",
    "scroll-marker-group",
    "selection",
    "slotted",
    "spelling-error",
    "target-text",
    "view-transition",
    "view-transition-group",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old"
};

/// The pseudo-classes and pseudo-elements that take arguments, as `params`
pub(super) static FUNCTIONAL_PSEUDOS: phf::Set<&str> = phf_set! {
    "active-view-transition-type",
    "cue",
    "cue-region",
    "dir",
    "has",
    "highlight",
    "host",
    "host-context",
    "is",
    "lang",
    "not",
    "nth-child",
    "nth-last-child",
    "nth-last-of-type",
    "nth-of-type",
    "part",
    "picker",
    "slotted",
    "state",
    "view-transition-group",
    "view-transition-image-pair",
    "view-transition-new",
    "view-transition-old",
    "where"
};

/// The pseudo-classes a `_name` key or a bare `selectors` key can name
pub(super) static PSEUDO_CLASSES: phf::Set<&str> = phf_set! {
    "active",
    "active-view-transition",
    "active-view-transition-type",
    "any-link",
    "autofill",
    "blank",
    "buffering",
    "checked",
    "current",
    "default",
    "defined",
    "dir",
    "disabled",
    "empty",
    "enabled",
    "first",
    "first-child",
    "first-of-type",
    "focus",
    "focus-visible",
    "focus-within",
    "fullscreen",
    "future",
    "has",
    "has-slotted",
    "host",
    "host-context",
    "hover",
    "in-range",
    "indeterminate",
    "invalid",
    "is",
    "lang",
    "last-child",
    "last-of-type",
    "left",
    "link",
    "local-link",
    "modal",
    "muted",
    "not",
    "nth-child",
    "nth-last-child",
    "nth-last-of-type",
    "nth-of-type",
    "only-child",
    "only-of-type",
    "open",
    "optional",
    "out-of-range",
    "past",
    "paused",
    "picture-in-picture",
    "placeholder-shown",
    "playing",
    "popover-open",
    "read-only",
    "read-write",
    "required",
    "right",
    "root",
    "scope",
    "seeking",
    "stalled",
    "state",
    "target",
    "target-current",
    "target-within",
    "user-invalid",
    "user-valid",
    "valid",
    "visited",
    "volume-locked",
    "where",
    "xr-overlay"
};

pub(super) static ZERO_PERCENT_FUNCTION: phf::Set<&str> = phf_set! {
    "abs(",
    "acos(",
    "asin(",
    "atan(",
    "atan2(",
    "calc(",
    "calc-size(",
    "clamp(",
    "cos(",
    "exp(",
    "hypot(",
    "log(",
    "max(",
    "min(",
    "mod(",
    "pow(",
    "rem(",
    "round(",
    "sign(",
    "sin(",
    "sqrt(",
    "tan(",
};

pub(super) static F_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"\s*,\s*"));
pub(super) static CSS_FUNCTION_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^[a-zA-Z-]+(\(.*\))"));

pub(super) static CSS_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"/\*[\s\S]*?\*/"));

pub(super) static F_DOT_RE: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"(\b|,)0\.(\d+)"));
pub(super) static DOT_ZERO_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"(\b|,)-?0\.0+([^\d])"));

pub(super) static COLOR_HASH: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"#([0-9a-zA-Z]+)"));
pub(super) static INNER_TRIM_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"\(\s*([^)]*?)\s*\)"));

pub(super) static RM_MINUS_ZERO_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r"-0(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg|\)|,)")
});

pub(super) static NUM_TRIM_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(r"(\d(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg)?)\s+(\d)")
});
pub(super) static ZERO_RE: LazyLock<Regex> = LazyLock::new(|| {
    compile_regex(
        r"(\b|,|\(|^|\s)-?0(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg)",
    )
});

pub(super) static F_RGBA_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"rgba\((\d+),(\d+),(\d+),(\d*\.?\d*)\)"));

pub(super) static F_RGB_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"rgb\((\d+),(\d+),(\d+)\)"));

pub(super) static N_BASE_ARRAY: [u8; 27] = *b"abcdefghijklmnopqrstuvwxyz_";

pub(super) static M_BASE_ARRAY: [u8; 37] = *b"abcdefghijklmnopqrstuvwxyz0123456789_";
