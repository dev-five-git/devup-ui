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
    "bgOrigin" => &["background-origin"],
    "bgPosition" => &["background-position"],
    "bgPositionX" => &["background-position-x"],
    "bgPositionY" => &["background-position-y"],
    "bgRepeat" => &["background-repeat"],
    "bgSize" => &["background-size"],
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
};

pub(super) static DOUBLE_SEPARATOR: phf::Set<&str> = phf_set! {
        "placeholder",
        "before",
        "after",
        "highlight",
        "view-transition",
        "view-transition-group",
        "view-transition-image-pair",
        "view-transition-new",
        "view-transition-old",
};

pub(super) static F_SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*,\s*").unwrap());
pub(super) static F_DOT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\b|,)0\.(\d+)").unwrap());
pub(super) static DOT_ZERO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(\b|,)-?0\.0+([^\d])").unwrap());

pub(super) static COLOR_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([0-9a-zA-Z]+)").unwrap());
pub(super) static ZERO_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(^|\s|\(|,)-?0(px|em|rem|vh|vw|%|dvh|dvw)").unwrap());
