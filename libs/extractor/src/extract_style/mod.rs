pub(super) mod constant;
pub(super) mod extract_css;
pub(super) mod extract_dynamic_style;
pub(super) mod extract_font_face;
pub(super) mod extract_import;
pub(super) mod extract_keyframes;
pub mod extract_static_style;
pub mod extract_style_value;
pub mod style_property;

use crate::extract_style::style_property::StyleProperty;
use css::style_selector::StyleSelector;
use std::borrow::Cow;

fn class_selector<'a>(
    selector: Option<&'a StyleSelector>,
    layer: Option<&str>,
) -> Option<Cow<'a, str>> {
    if css::atom_hoist::is_atom_hoist() {
        return Some(Cow::Owned(css::atom_name::selector_key(selector, layer)));
    }
    let selector = selector.map(StyleSelector::as_class_str);
    match layer {
        Some(layer) => Some(Cow::Owned(format!(
            "{}@layer {layer}",
            selector.as_deref().unwrap_or_default()
        ))),
        None => selector,
    }
}

pub trait ExtractStyleProperty {
    /// extract style properties
    fn extract(&self, filename: Option<&str>) -> StyleProperty;
}
