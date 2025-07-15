pub(super) mod constant;
pub(super) mod extract_css;
pub(super) mod extract_dynamic_style;
pub(super) mod extract_import;
pub(super) mod extract_static_style;
pub mod extract_style_value;

use crate::StyleProperty;

pub trait ExtractStyleProperty {
    /// extract style properties
    fn extract(&self) -> StyleProperty;
}
