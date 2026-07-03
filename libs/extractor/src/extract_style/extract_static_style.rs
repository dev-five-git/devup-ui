use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value},
    optimize_value::optimize_value,
    sheet_to_classname,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::{
    extract_style::{
        ExtractStyleProperty, constant::MAINTAIN_VALUE_PROPERTIES, style_property::StyleProperty,
    },
    utils::{convert_value, gcd},
};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash, Ord, PartialOrd, Default)]
pub enum ThemeTokenResolution {
    #[default]
    CssVariable,
    FirstValue,
}

#[derive(PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractStaticStyle {
    /// property
    pub property: String,
    /// fixed value
    pub value: String,
    /// responsive level
    pub level: u8,
    /// selector
    pub selector: Option<StyleSelector>,
    /// None is inf, 0 is first, 1 is second, etc
    pub style_order: Option<u8>,
    /// CSS layer name (from vanilla-extract `layer()`)
    pub layer: Option<String>,
    /// How theme tokens should be resolved when converting to CSS.
    pub theme_token_resolution: ThemeTokenResolution,
}

impl Debug for ExtractStaticStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractStaticStyle")
            .field("property", &self.property)
            .field("value", &self.value)
            .field("level", &self.level)
            .field("selector", &self.selector)
            .field("style_order", &self.style_order)
            .field("layer", &self.layer)
            .finish()
    }
}

impl ExtractStaticStyle {
    /// Normalize a static style value, shared by `new` and `new_basic`.
    ///
    /// When `apply_aspect_ratio` is `true`, the `aspect-ratio` value is reduced
    /// by its GCD (the behavior of `new`); when `false`, the raw value is kept
    /// verbatim for `MAINTAIN_VALUE_PROPERTIES` (the behavior of `new_basic`).
    fn normalize_static_value(property: &str, value: &str, apply_aspect_ratio: bool) -> String {
        // Build a `Cow<str>` so the common "kept-verbatim" `MAINTAIN_VALUE_PROPERTIES`
        // branch borrows `value` instead of allocating a throwaway `String` that is
        // immediately re-copied by `optimize_value` (which takes `&str` and owns its
        // own result). Only the aspect-ratio reduction and the `convert_value` branch
        // must own; both already produce owned `String`s. Byte-identical output.
        let normalized: Cow<str> = if MAINTAIN_VALUE_PROPERTIES.contains(property) {
            if apply_aspect_ratio && property == "aspect-ratio" && value.contains('/') {
                if let Some((a, b)) = value.split_once('/').and_then(|(a, b)| {
                    Some((a.trim().parse::<u32>().ok()?, b.trim().parse::<u32>().ok()?))
                }) {
                    let gcd = gcd(a, b);
                    Cow::Owned(format!("{}/{}", a / gcd, b / gcd))
                } else {
                    Cow::Borrowed(value)
                }
            } else {
                Cow::Borrowed(value)
            }
        } else {
            Cow::Owned(convert_value(value))
        };
        optimize_value(normalized.as_ref())
    }

    /// create a new `ExtractStaticStyle`
    pub fn new(property: &str, value: &str, level: u8, selector: Option<StyleSelector>) -> Self {
        Self {
            value: Self::normalize_static_value(property, value, true),
            property: property.to_string(),
            level,
            selector: selector.map(optimize_selector),
            style_order: None,
            layer: None,
            theme_token_resolution: ThemeTokenResolution::CssVariable,
        }
    }

    /// create a new `ExtractStaticStyle` with layer
    #[must_use]
    pub fn new_with_layer(
        property: &str,
        value: &str,
        level: u8,
        selector: Option<StyleSelector>,
        layer: Option<String>,
    ) -> Self {
        let mut style = Self::new(property, value, level, selector);
        style.layer = layer;
        style
    }

    #[must_use]
    pub fn new_basic(
        property: &str,
        value: &str,
        level: u8,
        selector: Option<StyleSelector>,
    ) -> Self {
        Self {
            value: Self::normalize_static_value(property, value, false),
            property: property.to_string(),
            level,
            selector,
            style_order: Some(0),
            layer: None,
            theme_token_resolution: ThemeTokenResolution::CssVariable,
        }
    }

    #[must_use]
    pub const fn with_theme_token_resolution(mut self, resolution: ThemeTokenResolution) -> Self {
        self.theme_token_resolution = resolution;
        self
    }

    /// Get the layer name
    #[must_use]
    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }

    #[must_use]
    pub const fn property(&self) -> &str {
        self.property.as_str()
    }

    #[must_use]
    pub const fn value(&self) -> &str {
        self.value.as_str()
    }

    #[must_use]
    pub const fn level(&self) -> u8 {
        self.level
    }

    #[must_use]
    pub const fn selector(&self) -> Option<&StyleSelector> {
        self.selector.as_ref()
    }

    #[must_use]
    pub const fn style_order(&self) -> Option<u8> {
        self.style_order
    }

    #[must_use]
    pub const fn theme_token_resolution(&self) -> ThemeTokenResolution {
        self.theme_token_resolution
    }
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let s = self.selector.as_ref().map(StyleSelector::as_class_str);
        // `self.value` is already the result of `optimize_value(convert_value(..))`
        // (computed in the constructors), so re-running convert_value + optimize_value
        // here is redundant. Only the multi-css optimization is not applied at construction.
        let v = if check_multi_css_optimize(&self.property) {
            std::borrow::Cow::Owned(optimize_multi_css_value(&self.value))
        } else {
            std::borrow::Cow::Borrowed(self.value.as_str())
        };
        StyleProperty::ClassName(sheet_to_classname(
            &self.property,
            self.level,
            Some(v.as_ref()),
            s.as_deref(),
            self.style_order,
            filename,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_static_style() {
        let style = ExtractStaticStyle::new("color", "red", 0, None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.value(), "red");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.style_order(), None);
        assert_eq!(style.layer(), None);
    }

    #[test]
    fn test_extract_static_style_with_layer() {
        let style =
            ExtractStaticStyle::new_with_layer("margin", "0", 0, None, Some("reset".to_string()));
        assert_eq!(style.property(), "margin");
        assert_eq!(style.value(), "0");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.layer(), Some("reset"));
    }

    #[test]
    fn test_extract_static_style_with_layer_none() {
        let style = ExtractStaticStyle::new_with_layer("color", "red", 0, None, None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.value(), "red");
        assert_eq!(style.layer(), None);
    }
}
