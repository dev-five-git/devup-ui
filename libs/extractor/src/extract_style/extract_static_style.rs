use std::borrow::Cow;
use std::fmt::{Debug, Formatter};

use css::{
    Naming,
    content_name::{AtomContent, ContentName},
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value},
    optimize_value::optimize_value,
    sheet_to_classname_owned,
    style_origin::Origin,
    style_selector::{StyleSelector, optimize_selector},
    theme_tokens::get_first_theme_token_value,
};

use crate::{
    extract_style::{
        ExtractStyleProperty,
        constant::{MAINTAIN_VALUE_PROPERTIES, TIME_PROPERTIES},
        style_property::StyleProperty,
    },
    utils::{convert_value, gcd},
};

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash, Ord, PartialOrd, Default)]
pub enum ThemeTokenResolution {
    #[default]
    CssVariable,
    FirstValue,
}

#[derive(Clone)]
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
    /// Which counter, if any, names the class; kept so the sheet names it
    /// again the same way.
    pub naming: Naming,
    /// Captured original allocation identity, retained for deferred sheet emission.
    pub counter_owner: css::CounterOwner,
    pub origin: Origin,
}

impl Debug for ExtractStaticStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ExtractStaticStyle");
        s.field("property", &self.property)
            .field("value", &self.value)
            .field("level", &self.level)
            .field("selector", &self.selector)
            .field("style_order", &self.style_order)
            .field("layer", &self.layer);
        if self.naming != Naming::Own {
            s.field("naming", &self.naming);
        }
        s.finish()
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
        let normalized: Cow<str> =
            if MAINTAIN_VALUE_PROPERTIES.contains(property) || property.starts_with("--") {
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
            } else if TIME_PROPERTIES.contains(property) {
                // A time is written in `ms`, never on the spacing scale
                value.parse::<f64>().map_or(Cow::Borrowed(value), |number| {
                    Cow::Owned(format!("{}ms", crate::utils::js_number_string(number)))
                })
            } else {
                convert_value(value)
            };
        // `optimize_value` returns `Cow` (borrowed when no optimization pass
        // fires); this constructor stores an owned `String`, so materialize it
        // here — a borrowed result costs exactly the one copy it always did.
        optimize_value(normalized.as_ref()).into_owned()
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
            naming: Naming::Own,
            counter_owner: crate::sparse_sites::counter_owner(),
            origin: crate::style_origin::current(),
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
            naming: Naming::Own,
            counter_owner: crate::sparse_sites::counter_owner(),
            origin: crate::style_origin::current(),
        }
    }

    #[must_use]
    pub const fn with_theme_token_resolution(mut self, resolution: ThemeTokenResolution) -> Self {
        self.theme_token_resolution = resolution;
        self
    }

    #[must_use]
    pub const fn with_naming(mut self, naming: Naming) -> Self {
        self.naming = naming;
        self
    }

    /// Get the layer name
    #[must_use]
    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }

    /// The selector part of the class name key, holding the layer so a layered
    /// declaration never shares a class with an unlayered one
    #[must_use]
    pub fn class_selector(&self) -> Option<Cow<'_, str>> {
        super::class_selector(self.selector.as_ref(), self.layer.as_deref())
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

    /// Effective value shared by generated class identity and emitted declaration.
    pub fn resolved_value(&self) -> Cow<'_, str> {
        match self.theme_token_resolution() {
            ThemeTokenResolution::CssVariable => Cow::Borrowed(&self.value),
            ThemeTokenResolution::FirstValue => {
                get_first_theme_token_value(&self.property, &self.value)
                    .map_or(Cow::Borrowed(&self.value), Cow::Owned)
            }
        }
    }

    #[must_use]
    pub fn content_name(&self) -> ContentName {
        let value = self.effective_value();
        self.atom_content(&value).content()
    }

    #[must_use]
    pub fn effective_value(&self) -> String {
        if self.property == "typography" {
            return css::content_typography::identity(&self.value, self.level);
        }
        let value = self.resolved_value();
        let value = if self.property != "content" && check_multi_css_optimize(&self.property) {
            optimize_multi_css_value(&value).into_owned()
        } else {
            value.into_owned()
        };
        css::content_value::emitted(&value).into_owned()
    }

    fn atom_content<'a>(&'a self, value: &'a str) -> AtomContent<'a> {
        AtomContent {
            property: &self.property,
            value: Some(value),
            naming: self.naming,
            level: self.level,
            order: self.style_order.unwrap_or(255),
            selector: self.selector.as_ref(),
            layer: self.layer.as_deref(),
            dynamic: false,
        }
    }
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let value = self.effective_value();
        StyleProperty::ClassName(sheet_to_classname_owned(
            &self.atom_content(&value),
            filename,
            self.counter_owner,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_strings_keep_quotes_when_effective_values_name_and_emit_them() {
        for value in ["\"\"", "\"red\"", "'hello world'"] {
            let style = ExtractStaticStyle::new("content", value, 0, None);
            assert_eq!(style.effective_value(), value);
            assert_ne!(
                style.content_name(),
                ExtractStaticStyle::new("content", value.trim_matches(['\'', '"']), 0, None)
                    .content_name()
            );
        }
    }

    #[test]
    fn test_extract_static_style() {
        let style = ExtractStaticStyle::new("color", "red", 0, None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.value(), "red");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.style_order(), None);
        assert_eq!(style.layer(), None);
        assert_eq!(
            ExtractStaticStyle::new("--columns", "4", 0, None).value(),
            "4"
        );
        assert_eq!(
            ExtractStaticStyle::new("padding", "4", 0, None).value(),
            "16px"
        );
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
