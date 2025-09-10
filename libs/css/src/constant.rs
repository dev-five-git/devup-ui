use std::collections::HashMap;

use once_cell::sync::Lazy;
use phf::{phf_map, phf_set};
use regex::Regex;

pub(super) static SELECTOR_ORDER_MAP: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (idx, selector) in [
        "hover",
        "focus-visible",
        "focus",
        "active",
        "selected",
        "disabled",
    ]
    .into_iter()
    .enumerate()
    {
        map.insert(format!(":{selector}"), idx as u8);
    }
    map
});

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
pub(super) static OPTIMIZE_MULTI_CSS_VALUE_PROPERTY: phf::Set<&str> = phf_set! {
    "font-family",
    "src",
    "content",
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
    "marker",
    "picker-icon",
    "placeholder",
    "scroll-marker",
    "scroll-marker-group",
    "selection",
    "spelling-error",
    "target-text",
    "view-transition"
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

pub(super) static F_SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*,\s*").unwrap());
pub(super) static CSS_FUNCTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z-]+(\(.*\))").unwrap());
pub(super) static CHECK_QUOTES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[()\s]").unwrap());

pub(super) static CSS_COMMENT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"/\*[\s\S]*?\*/").unwrap());

pub(super) static F_DOT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\b|,)0\.(\d+)").unwrap());
pub(super) static DOT_ZERO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\b|,)-?0\.0+([^\d])").unwrap());

pub(super) static COLOR_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([0-9a-zA-Z]+)").unwrap());
pub(super) static INNER_TRIM_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\(\s*([^)]*?)\s*\)").unwrap());

pub(super) static RM_MINUS_ZERO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-0(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg|\)|,)").unwrap()
});

pub(super) static NUM_TRIM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg)?)\s+(\d)")
        .unwrap()
});
pub(super) static ZERO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\b|,|\(|^|\s)-?0(px|em|rem|vh|vw|%|dvh|dvw|vmax|vmin|mm|cm|in|pt|pc|lh|ic|deg)")
        .unwrap()
});

pub(super) static F_RGBA_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"rgba\((\d+),(\d+),(\d+),(\d*\.?\d*)\)").unwrap());

pub(super) static F_RGB_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"rgb\((\d+),(\d+),(\d+)\)").unwrap());

pub(super) static N_BASE_ARRAY: [char; 26] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub(super) static M_BASE_ARRAY: [char; 36] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];
