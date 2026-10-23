use std::fmt::{Debug, Formatter};

use css::{
    sheet_to_classname, sheet_to_variable_name,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};

#[derive(PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,

    /// selector
    selector: Option<StyleSelector>,

    pub(super) style_order: Option<u8>,

    /// Whether the value had `!important` that was stripped from the identifier
    important: bool,

    pub(crate) layer: Option<String>,
}

impl Debug for ExtractDynamicStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ExtractDynamicStyle");
        s.field("property", &self.property)
            .field("level", &self.level)
            .field("identifier", &self.identifier)
            .field("selector", &self.selector)
            .field("style_order", &self.style_order);
        if self.important {
            s.field("important", &self.important);
        }
        if let Some(layer) = &self.layer {
            s.field("layer", layer);
        }
        s.finish()
    }
}

/// `(closing symbol, " !important" + closing symbol)` probe pairs for `strip_important`.
const IMPORTANT_SUFFIXES: [(&str, &str); 4] = [
    ("", " !important"),
    ("`", " !important`"),
    ("\"", " !important\""),
    ("'", " !important'"),
];

/// Strip ` !important` from a dynamic style identifier, returning the cleaned
/// identifier and whether `!important` was found.
///
/// Takes the identifier by value so the overwhelmingly common no-match path can
/// move the owned string straight through without allocating a fresh copy.
///
/// Handles JS expression code produced by `expression_to_code`, where the
/// `!important` text appears before a closing delimiter (backtick, quote) or
/// at the very end of the string.
fn strip_important(identifier: String) -> (String, bool) {
    // Overwhelmingly common case: no `!important` anywhere. A single substring
    // scan short-circuits before the 4-probe `strip_suffix` loop (each of which
    // scans from the string end). Byte-identical: any string that would match a
    // suffix necessarily contains `!important`, so no match is ever skipped.
    if !identifier.contains("!important") {
        return (identifier, false);
    }
    for (str_symbol, suffix) in IMPORTANT_SUFFIXES {
        if let Some(base) = identifier.strip_suffix(suffix) {
            // The bare-identifier case (`str_symbol == ""`, the most common) needs
            // no formatting machinery — a plain owned copy of `base` suffices.
            if str_symbol.is_empty() {
                return (base.to_string(), true);
            }
            return (format!("{base}{str_symbol}"), true);
        }
    }
    // No `!important` suffix — move the owned string through unchanged (zero alloc).
    (identifier, false)
}

/// `identifier`, the code the element runs, without the statement's semicolon;
/// only a template holding the whole CSS value loses the `;` ending it. The
/// code is not optimized as CSS, as that would rewrite the literals it passes on
fn runtime_code(identifier: &str) -> String {
    let code = identifier.trim();
    let code = code.strip_suffix(';').unwrap_or(code);
    match code
        .strip_prefix('`')
        .and_then(|code| code.strip_suffix('`'))
        .filter(|value| value.ends_with(';'))
    {
        Some(value) => format!("`{}`", value.trim_end_matches(';')),
        None => code.to_string(),
    }
}

impl ExtractDynamicStyle {
    /// create a new `ExtractDynamicStyle`
    pub fn new(
        property: &str,
        level: u8,
        identifier: &str,
        selector: Option<StyleSelector>,
    ) -> Self {
        // `optimize_value` returns `Cow`; `strip_important` takes ownership (and
        // the struct stores the `String`), so materialize the owned form here.
        let (identifier, important) = strip_important(runtime_code(identifier));
        Self {
            property: property.to_string(),
            level,
            identifier,
            selector: selector.map(optimize_selector),
            style_order: None,
            important,
            layer: None,
        }
    }

    pub const fn property(&self) -> &str {
        self.property.as_str()
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn selector(&self) -> Option<&StyleSelector> {
        self.selector.as_ref()
    }

    pub const fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    pub(crate) const fn identifier_mut(&mut self) -> &mut String {
        &mut self.identifier
    }

    pub const fn style_order(&self) -> Option<u8> {
        self.style_order
    }

    pub const fn important(&self) -> bool {
        self.important
    }

    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let selector = super::class_selector(self.selector.as_ref(), self.layer());
        let ordinary_class = (!css::atom_hoist::is_atom_hoist()).then(|| {
            sheet_to_classname(
                self.property.as_str(),
                self.level,
                self.important.then_some("!important"),
                selector.as_deref(),
                self.style_order,
                filename,
            )
        });
        let variable_name =
            sheet_to_variable_name(self.property.as_str(), self.level, selector.as_deref());
        let declaration = {
            format!(
                "var({variable_name}){}",
                if self.important { " !important" } else { "" }
            )
        };
        StyleProperty::Variable {
            class_name: ordinary_class.unwrap_or_else(|| {
                sheet_to_classname(
                    self.property.as_str(),
                    self.level,
                    Some(&declaration),
                    selector.as_deref(),
                    self.style_order,
                    filename,
                )
            }),
            variable_name,
            identifier: self.identifier.clone(),
        }
    }
}

#[cfg(test)]
#[path = "dynamic_style_tests.rs"]
mod tests;
