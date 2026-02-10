pub mod theme;

use crate::theme::Theme;
use css::{
    merge_selector,
    style_selector::{AtRuleKind, StyleSelector},
};
use extractor::extract_style::ExtractStyleProperty;
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::extract_style::style_property::StyleProperty;
use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering::Equal;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write;
use std::sync::LazyLock;

trait ExtractStyle {
    fn extract(&self) -> String;
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetProperty {
    #[serde(rename = "c")]
    pub class_name: String,
    #[serde(rename = "p")]
    pub property: String,
    #[serde(rename = "v")]
    pub value: String,
    #[serde(rename = "s")]
    pub selector: Option<StyleSelector>,
    /// CSS layer name (from vanilla-extract layer())
    #[serde(rename = "l", skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetKeyframes {
    pub name: String,
    pub keyframes: BTreeMap<String, BTreeSet<StyleSheetProperty>>,
}

impl PartialOrd for StyleSheetProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StyleSheetProperty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.selector.is_some(), other.selector.is_some()) {
            (true, true) => match self.selector.cmp(&other.selector) {
                Equal => match self.property.cmp(&other.property) {
                    Equal => self.value.cmp(&other.value),
                    val => val,
                },
                val => val,
            },
            (false, false) => match self.property.cmp(&other.property) {
                Equal => self.value.cmp(&other.value),
                prop => prop,
            },
            (a, b) => a.cmp(&b),
        }
    }
}

impl ExtractStyle for StyleSheetProperty {
    fn extract(&self) -> String {
        format!(
            "{}{{{}:{}}}",
            merge_selector(&self.class_name, self.selector.as_ref()),
            self.property,
            convert_theme_variable_value(&self.value)
        )
    }
}

static VAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$[\w.]+").unwrap());
static INTERFACE_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_$][a-zA-Z0-9_$]*$").unwrap());

/// Cached header string — computed once from compile-time included package.json
static HEADER: LazyLock<String> = LazyLock::new(|| {
    format!(
        "/*! devup-ui v{version} | Apache License 2.0 | https://devup-ui.com */",
        version = include_str!("../../../bindings/devup-ui-wasm/package.json")
            .lines()
            .find(|line| line.contains("\"version\""))
            .unwrap()
            .split(":")
            .nth(1)
            .unwrap()
            .trim()
            .replace("\"", ""),
    )
});

fn convert_interface_key(key: &str) -> String {
    if INTERFACE_KEY_RE.is_match(key) {
        key.to_string()
    } else {
        format!("[`{}`]", key.replace("`", "\\`"))
    }
}

fn convert_theme_variable_value(value: &str) -> Cow<'_, str> {
    if value.contains('$') {
        Cow::Owned(
            VAR_RE
                .replace_all(value, |caps: &regex::Captures| {
                    format!("var(--{})", &caps[0][1..].replace('.', "-"))
                })
                .into_owned(),
        )
    } else {
        Cow::Borrowed(value)
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize, Ord, PartialOrd)]
pub struct StyleSheetCss {
    pub css: String,
}

impl ExtractStyle for StyleSheetCss {
    fn extract(&self) -> String {
        self.css.clone()
    }
}

type PropertyMap = BTreeMap<u8, BTreeMap<u8, HashSet<StyleSheetProperty>>>;
type KeyframesMap = BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>>;

fn deserialize_btree_map_u8<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, PropertyMap>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut result: BTreeMap<String, PropertyMap> = BTreeMap::new();
    for (key, value) in BTreeMap::<
        String,
        BTreeMap<String, BTreeMap<String, HashSet<StyleSheetProperty>>>,
    >::deserialize(deserializer)?
    {
        let mut tmp_map: PropertyMap = BTreeMap::new();

        for (key, value) in value.into_iter() {
            let mut inner_tmp_map = BTreeMap::new();
            for (key, value) in value.into_iter() {
                inner_tmp_map.insert(key.parse().map_err(Error::custom)?, value);
            }
            tmp_map.insert(key.parse().map_err(Error::custom)?, inner_tmp_map);
        }

        result.insert(key, tmp_map);
    }

    Ok(result)
}
#[derive(Default, Deserialize, Serialize, Debug)]
pub struct StyleSheet {
    #[serde(deserialize_with = "deserialize_btree_map_u8", default)]
    pub properties: BTreeMap<String, PropertyMap>,
    #[serde(default)]
    pub css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    #[serde(default)]
    pub keyframes: KeyframesMap,
    #[serde(default)]
    pub global_css_files: BTreeSet<String>,
    #[serde(default)]
    pub imports: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub font_faces: BTreeMap<String, BTreeSet<BTreeMap<String, String>>>,
    #[serde(skip)]
    pub theme: Theme,
}

impl StyleSheet {
    #[allow(clippy::too_many_arguments)]
    pub fn add_property(
        &mut self,
        class_name: &str,
        property: &str,
        level: u8,
        value: &str,
        selector: Option<&StyleSelector>,
        style_order: Option<u8>,
        filename: Option<&str>,
    ) -> bool {
        self.add_property_with_layer(
            class_name,
            property,
            level,
            value,
            selector,
            style_order,
            filename,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_property_with_layer(
        &mut self,
        class_name: &str,
        property: &str,
        level: u8,
        value: &str,
        selector: Option<&StyleSelector>,
        style_order: Option<u8>,
        filename: Option<&str>,
        layer: Option<&str>,
    ) -> bool {
        // register global css file for cache
        if let Some(StyleSelector::Global(_, file)) = selector {
            self.global_css_files.insert(file.clone());
        }

        self.properties
            .entry(filename.unwrap_or_default().to_string())
            .or_default()
            .entry(style_order.unwrap_or(255))
            .or_default()
            .entry(level)
            .or_default()
            .insert(StyleSheetProperty {
                class_name: class_name.to_string(),
                property: property.to_string(),
                value: value.to_string(),
                selector: selector.cloned(),
                layer: layer.map(|s| s.to_string()),
            })
    }

    pub fn add_import(&mut self, file: &str, import: &str) {
        self.global_css_files.insert(file.to_string());
        self.imports
            .entry(file.to_string())
            .or_default()
            .insert(import.to_string());
    }

    pub fn add_font_face(&mut self, file: &str, properties: &BTreeMap<String, String>) {
        self.global_css_files.insert(file.to_string());
        self.font_faces
            .entry(file.to_string())
            .or_default()
            .insert(properties.clone());
    }

    pub fn add_css(&mut self, file: &str, css: &str) -> bool {
        self.global_css_files.insert(file.to_string());
        self.css
            .entry(file.to_string())
            .or_default()
            .insert(StyleSheetCss {
                css: css.to_string(),
            })
    }

    pub fn add_keyframes(
        &mut self,
        name: &str,
        keyframes: BTreeMap<String, Vec<(String, String)>>,
        filename: Option<&str>,
    ) -> bool {
        let map = self
            .keyframes
            .entry(filename.unwrap_or_default().to_string())
            .or_default()
            .entry(name.to_string())
            .or_default();
        if map == &keyframes {
            return false;
        }
        map.clear();
        map.extend(keyframes);
        true
    }

    pub fn rm_global_css(&mut self, file: &str, single_css: bool) -> bool {
        if !self.global_css_files.contains(file) {
            return false;
        }
        self.global_css_files.remove(file);
        self.css.remove(file);

        self.font_faces.remove(file);
        let property_key = if single_css { "" } else { file }.to_string();

        for map in self
            .properties
            .entry(property_key.clone())
            .or_default()
            .values_mut()
        {
            for props in map.values_mut() {
                props.retain(|prop| {
                    if let Some(StyleSelector::Global(_, f)) = prop.selector.as_ref() {
                        f != file
                    } else {
                        true
                    }
                });
            }
            // remove empty map
            if map.iter().all(|(_, v)| v.is_empty()) {
                map.clear();
            }
        }
        if self
            .properties
            .get(&property_key)
            .and_then(|v| if v.is_empty() { None } else { Some(()) })
            .is_none()
        {
            self.properties.remove(&property_key);
        }
        true
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn update_styles(
        &mut self,
        styles: &HashSet<ExtractStyleValue>,
        filename: &str,
        single_css: bool,
    ) -> (bool, bool) {
        let mut collected = false;
        let mut updated_base_style = false;
        for style in styles.iter() {
            match style {
                ExtractStyleValue::Static(st) => {
                    if let Some(StyleProperty::ClassName(cls)) =
                        style.extract(if !single_css { Some(filename) } else { None })
                        && self.add_property_with_layer(
                            &cls,
                            st.property(),
                            st.level(),
                            st.value(),
                            st.selector(),
                            st.style_order(),
                            if !single_css { Some(filename) } else { None },
                            st.layer(),
                        )
                    {
                        collected = true;
                        if st.style_order() == Some(0) {
                            updated_base_style = true;
                        }
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    if let Some(StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    }) = style.extract(if !single_css { Some(filename) } else { None })
                        && self.add_property(
                            &class_name,
                            dy.property(),
                            dy.level(),
                            &format!("var({})", variable_name),
                            dy.selector(),
                            dy.style_order(),
                            if !single_css { Some(filename) } else { None },
                        )
                    {
                        collected = true;
                        if dy.style_order() == Some(0) {
                            updated_base_style = true;
                        }
                    }
                }

                ExtractStyleValue::Keyframes(keyframes) => {
                    if self.add_keyframes(
                        &keyframes
                            .extract(if !single_css { Some(filename) } else { None })
                            .to_string(),
                        keyframes
                            .keyframes
                            .iter()
                            .map(|(key, value)| {
                                (
                                    key.clone(),
                                    value
                                        .iter()
                                        .map(|style| {
                                            (
                                                style.property().to_string(),
                                                style.value().to_string(),
                                            )
                                        })
                                        .collect::<Vec<(String, String)>>(),
                                )
                            })
                            .collect(),
                        if !single_css { Some(filename) } else { None },
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Css(cs) => {
                    if self.add_css(&cs.file, &cs.css) {
                        // update global css
                        updated_base_style = true;
                    }
                }
                ExtractStyleValue::Typography(_) => {}
                ExtractStyleValue::Import(st) => {
                    self.add_import(&st.file, &st.url);
                }
                ExtractStyleValue::FontFace(font) => {
                    self.add_font_face(&font.file, &font.properties);
                }
            }
        }
        (collected, updated_base_style)
    }

    pub fn create_interface(
        &self,
        package_name: &str,
        color_interface_name: &str,
        typography_interface_name: &str,
        theme_interface_name: &str,
    ) -> String {
        let mut color_keys = BTreeSet::new();
        let mut typography_keys = BTreeSet::new();
        let mut theme_keys = BTreeSet::new();
        for color_theme in self.theme.colors.values() {
            color_theme.interface_keys().for_each(|key| {
                color_keys.insert(key.clone());
            });
        }
        self.theme.typography.keys().for_each(|key| {
            typography_keys.insert(key.clone());
        });

        self.theme.colors.keys().for_each(|key| {
            theme_keys.insert(key.clone());
        });

        if color_keys.is_empty() && typography_keys.is_empty() {
            String::new()
        } else {
            format!(
                "import \"{}\";declare module \"{}\"{{interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}}}",
                package_name,
                package_name,
                color_interface_name,
                color_keys
                    .into_iter()
                    .map(|key| format!("{}:null", convert_interface_key(&format!("${key}"))))
                    .collect::<Vec<_>>()
                    .join(";"),
                typography_interface_name,
                typography_keys
                    .into_iter()
                    .map(|key| format!("{}:null", convert_interface_key(&key)))
                    .collect::<Vec<_>>()
                    .join(";"),
                theme_interface_name,
                theme_keys
                    .into_iter()
                    .map(|key| format!("{}:null", convert_interface_key(&key)))
                    .collect::<Vec<_>>()
                    .join(";")
            )
        }
    }
    fn create_style(&self, map: &BTreeMap<u8, HashSet<StyleSheetProperty>>) -> String {
        self.create_style_with_layers(map, &mut BTreeMap::new())
    }

    fn create_style_with_layers(
        &self,
        map: &BTreeMap<u8, HashSet<StyleSheetProperty>>,
        layered_styles: &mut BTreeMap<String, Vec<(String, String, String)>>, // layer -> Vec<(selector, property, value)>
    ) -> String {
        let mut current_css = String::new();
        for (level, props) in map.iter() {
            let (mut global_props, rest): (Vec<_>, Vec<_>) = props
                .iter()
                .partition(|prop| matches!(prop.selector, Some(StyleSelector::Global(_, _))));
            global_props.sort();
            let (mut at_rules, mut sorted_props): (Vec<_>, Vec<_>) = rest
                .into_iter()
                .partition(|prop| matches!(prop.selector, Some(StyleSelector::At { .. })));
            sorted_props.sort();
            at_rules.sort();
            let at_rules = {
                let mut map: BTreeMap<(AtRuleKind, &String), Vec<_>> = BTreeMap::new();
                for prop in at_rules {
                    if let Some(StyleSelector::At { kind, query, .. }) = &prop.selector {
                        map.entry((*kind, query))
                            .or_insert_with(Vec::new)
                            .push(prop);
                    }
                }
                map
            };

            let break_point = if *level == 0 {
                None
            } else {
                Some(
                    self.theme
                        .breakpoints
                        .iter()
                        .enumerate()
                        .find(|(idx, _)| (*idx as u8) == *level)
                        .map(|(_, bp)| *bp)
                        .unwrap_or_else(|| self.theme.breakpoints.last().cloned().unwrap_or(0)),
                )
            };

            if !global_props.is_empty() {
                // Separate layered and non-layered global props
                let (layered_props, non_layered_props): (Vec<_>, Vec<_>) = global_props
                    .into_iter()
                    .partition(|prop| prop.layer.is_some());

                // Collect layered props for later processing
                for prop in layered_props {
                    if let Some(layer) = &prop.layer
                        && let Some(StyleSelector::Global(selector, _)) = &prop.selector
                    {
                        layered_styles.entry(layer.clone()).or_default().push((
                            selector.clone(),
                            prop.property.clone(),
                            prop.value.clone(),
                        ));
                    }
                }

                // Process non-layered global props as before
                if !non_layered_props.is_empty() {
                    let mut selector_map: BTreeMap<_, Vec<_>> = BTreeMap::new();
                    for prop in non_layered_props {
                        if let Some(StyleSelector::Global(selector, _)) = &prop.selector {
                            selector_map.entry(selector.clone()).or_default().push(prop);
                        }
                    }
                    if let Some(break_point) = break_point {
                        write!(current_css, "@media(min-width:{break_point}px){{").unwrap();
                    }
                    for (selector, props) in selector_map {
                        current_css.push_str(&selector);
                        current_css.push('{');
                        let mut first = true;
                        for prop in props {
                            if !first {
                                current_css.push(';');
                            }
                            first = false;
                            current_css.push_str(&prop.property);
                            current_css.push(':');
                            current_css.push_str(&prop.value);
                        }
                        current_css.push('}');
                    }
                    if break_point.is_some() {
                        current_css.push('}');
                    }
                }
            }

            if !sorted_props.is_empty() {
                if let Some(break_point) = break_point {
                    write!(current_css, "@media(min-width:{break_point}px){{").unwrap();
                }
                for prop in sorted_props {
                    current_css.push_str(&prop.extract());
                }
                if break_point.is_some() {
                    current_css.push('}');
                }
            }
            for ((kind, query), props) in at_rules {
                if let Some(break_point) = break_point {
                    match kind {
                        AtRuleKind::Media => {
                            write!(
                                current_css,
                                "@media(min-width:{break_point}px)and {query}{{"
                            )
                            .unwrap();
                        }
                        AtRuleKind::Supports => {
                            write!(
                                current_css,
                                "@media(min-width:{break_point}px){{@supports{query}{{"
                            )
                            .unwrap();
                        }
                        AtRuleKind::Container => {
                            write!(
                                current_css,
                                "@media(min-width:{break_point}px){{@container{query}{{"
                            )
                            .unwrap();
                        }
                        AtRuleKind::Layer => {
                            write!(
                                current_css,
                                "@media(min-width:{break_point}px){{@layer {query}{{"
                            )
                            .unwrap();
                        }
                    }
                    for prop in props {
                        current_css.push_str(&prop.extract());
                    }
                    match kind {
                        AtRuleKind::Media => current_css.push('}'),
                        _ => current_css.push_str("}}"),
                    }
                } else {
                    write!(current_css, "@{kind}").unwrap();
                    if query.starts_with('(') {
                        write!(current_css, "{query}{{").unwrap();
                    } else {
                        write!(current_css, " {query}{{").unwrap();
                    }
                    for prop in props {
                        current_css.push_str(&prop.extract());
                    }
                    current_css.push('}');
                }
            }
        }
        current_css
    }

    #[inline]
    fn create_header(&self) -> &'static str {
        &HEADER
    }

    pub fn create_css(&self, filename: Option<&str>, import_main_css: bool) -> String {
        let mut css = String::with_capacity(4096);
        css.push_str(self.create_header());
        for import in self.imports.values().flatten() {
            if import.starts_with('"') {
                write!(css, "@import {import};").unwrap();
            } else {
                write!(css, "@import \"{import}\";").unwrap();
            }
        }

        let write_global = filename.is_none();

        if write_global {
            let mut style_orders: BTreeSet<u8> = BTreeSet::new();
            let mut base_styles = BTreeMap::<u8, HashSet<StyleSheetProperty>>::new();
            self.properties.values().for_each(|map| {
                style_orders.extend(map.iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| *k));
                if let Some(_base_styles) = map.get(&0) {
                    _base_styles.iter().for_each(|prop| {
                        base_styles
                            .entry(*prop.0)
                            .or_default()
                            .extend(prop.1.clone());
                    });
                }
            });
            // default
            style_orders.remove(&255);
            // base style

            let theme_css = self.theme.to_css();
            let mut layers_vec = Vec::new();
            if style_orders.remove(&0) {
                layers_vec.push("b".to_string());
            }
            if !theme_css.is_empty() {
                layers_vec.push("t".to_string());
            }
            layers_vec.extend(style_orders.iter().map(|v| format!("o{v}")));
            if !layers_vec.is_empty() {
                css.push_str("@layer ");
                let mut first = true;
                for layer in &layers_vec {
                    if !first {
                        css.push(',');
                    }
                    first = false;
                    css.push_str(layer);
                }
                css.push(';');
            }
            if !theme_css.is_empty() {
                write!(css, "@layer t{{{theme_css}}}").unwrap();
            }
            for (_, font_faces) in self.font_faces.iter() {
                for font_face in font_faces.iter() {
                    css.push_str("@font-face{");
                    let mut first = true;
                    for (key, value) in font_face.iter() {
                        if !first {
                            css.push(';');
                        }
                        first = false;
                        write!(css, "{key}:{value}").unwrap();
                    }
                    css.push('}');
                }
            }

            // global css
            for (_, _css) in self.css.iter() {
                for _css in _css.iter() {
                    css.push_str(&_css.css);
                }
            }

            // Collect layered styles while creating base CSS
            let mut layered_styles: BTreeMap<String, Vec<(String, String, String)>> =
                BTreeMap::new();
            let base_css = self.create_style_with_layers(&base_styles, &mut layered_styles);
            if !base_css.is_empty() {
                write!(css, "@layer b{{{base_css}}}").unwrap();
            }

            // Generate @layer declarations and wrapped styles for custom layers
            if !layered_styles.is_empty() {
                // Add layer declarations
                css.push_str("@layer ");
                let mut first = true;
                for name in layered_styles.keys() {
                    if !first {
                        css.push(',');
                    }
                    first = false;
                    css.push_str(name);
                }
                css.push(';');

                // Generate styles wrapped in @layer blocks
                for (layer_name, styles) in layered_styles {
                    // Group by selector
                    let mut selector_map: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
                    for (selector, property, value) in styles {
                        selector_map
                            .entry(selector)
                            .or_default()
                            .push((property, value));
                    }

                    write!(css, "@layer {layer_name}{{").unwrap();
                    for (selector, props) in selector_map {
                        css.push_str(&selector);
                        css.push('{');
                        let mut first = true;
                        for (p, v) in props {
                            if !first {
                                css.push(';');
                            }
                            first = false;
                            write!(css, "{p}:{v}").unwrap();
                        }
                        css.push('}');
                    }
                    css.push('}');
                }
            }
        } else {
            // avoid inline import issue (vite plugin)
            if import_main_css {
                // import global css
                css.push_str("@import \"./devup-ui.css\";");
            }
        }

        if let Some(keyframes) = self.keyframes.get(filename.unwrap_or_default()) {
            for (name, map) in keyframes {
                write!(css, "@keyframes {name}{{").unwrap();
                for (key, props) in map.iter() {
                    write!(css, "{key}{{").unwrap();
                    let mut first = true;
                    for (k, v) in props.iter() {
                        if !first {
                            css.push(';');
                        }
                        first = false;
                        write!(css, "{k}:{v}").unwrap();
                    }
                    css.push('}');
                }
                css.push('}');
            }
        }

        // order
        if let Some(maps) = self.properties.get(filename.unwrap_or_default()) {
            for (style_order, map) in maps.iter() {
                if *style_order == 0 {
                    // base style was created in global css
                    continue;
                }
                let current_css = self.create_style(map);

                if !current_css.is_empty() {
                    // order style 255 is user css
                    if *style_order == 255 {
                        css.push_str(&current_css);
                    } else {
                        write!(css, "@layer o{style_order}{{{current_css}}}").unwrap();
                    }
                }
            }
        }
        css
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::{ColorTheme, Typography};

    use super::*;
    use extractor::{ExtractOption, extract};
    use insta::assert_debug_snapshot;

    use rstest::rstest;

    #[rstest]
    #[case("1px", "1px")]
    #[case("$var", "var(--var)")]
    #[case("$var $var", "var(--var) var(--var)")]
    #[case("1px solid $red", "1px solid var(--red)")]
    // Test dot notation theme variables (e.g., $primary.100)
    // Dots should be converted to dashes for CSS variable names
    #[case("$primary.100", "var(--primary-100)")]
    #[case("$gray.200 $blue.500", "var(--gray-200) var(--blue-500)")]
    #[case("1px solid $border.primary", "1px solid var(--border-primary)")]
    // Test deep nested dot notation
    #[case("$color.brand.primary.100", "var(--color-brand-primary-100)")]
    fn test_convert_theme_variable_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(convert_theme_variable_value(input), expected);
    }

    #[test]
    fn test_create_css_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None, None);
        sheet.add_property("test", "background", 1, "some", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, None, None);
        sheet.add_property("test", "border-color", 0, "red", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }
    #[test]
    fn test_create_css_with_selector_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&"hover".into()),
            None,
            None,
        );
        sheet.add_property("test", "background-color", 1, "some", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None, None);
        sheet.add_property(
            "test",
            "background-color",
            1,
            "some",
            Some(&"hover".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None, None);
        sheet.add_property("test", "background", 1, "some", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }
    #[test]
    fn test_create_css_with_basic_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, Some(0), None);
        sheet.add_property("test", "background", 1, "some", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, None, None);
        sheet.add_property("test", "border-color", 0, "red", None, Some(0), None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, Some(0), None);
        sheet.add_property("test", "display", 0, "block", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_create_css_with_selector_and_basic_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&"hover".into()),
            None,
            None,
        );
        sheet.add_property("test", "background-color", 1, "some", None, Some(0), None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, Some(0), None);
        sheet.add_property("test", "display", 0, "none", None, None, None);
        sheet.add_property("test", "display", 2, "flex", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_import_css() {
        let sheet = StyleSheet::default();
        assert_debug_snapshot!(
            sheet
                .create_css(Some("index.tsx"), true)
                .split("*/")
                .nth(1)
                .unwrap()
        );
    }

    #[test]
    fn test_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin", 1, "40px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_css("test.tsx", "div {display:flex;}");
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin", 2, "40px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"hover".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&"active".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("hover")),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"*:hover &".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&["theme-dark", "hover"].into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&["wrong", "hover"].into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"*[disabled='true'] &:hover".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"&[disabled='true']".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"&[disabled='true'], &[disabled='true']".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_reset_global_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_css("test.tsx", "div {display:flex;}");
        sheet.add_css("test2.tsx", "div {display:flex;}");
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        sheet.rm_global_css("test.tsx", true);

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        sheet.rm_global_css("wrong.tsx", true);

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_style_order_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin-left", 0, "40px", None, Some(1), None);
        sheet.add_property("test", "margin-right", 0, "40px", None, Some(1), None);

        sheet.add_property("test", "margin-left", 1, "40px", None, Some(1), None);
        sheet.add_property("test", "margin-right", 1, "40px", None, Some(1), None);
        sheet.add_property("test", "margin-left", 1, "44px", None, Some(1), None);
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1), None);
        sheet.add_property("test", "margin-left", 1, "40px", None, Some(1), None);
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1), None);
        sheet.add_property("test", "margin-left", 1, "44px", None, Some(1), None);
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1), None);
        sheet.add_property("test", "margin-left", 1, "50px", None, Some(2), None);
        sheet.add_property("test", "margin-right", 1, "50px", None, Some(2), None);
        sheet.add_property("test", "margin-left", 1, "60px", None, None, None);
        sheet.add_property("test", "margin-right", 1, "60px", None, None, None);
        sheet.add_property("test", "margin-left", 0, "70px", None, None, None);
        sheet.add_property("test", "margin-right", 0, "70px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background", 0, "red", None, Some(3), None);
        sheet.add_property("test", "background", 0, "blue", None, Some(17), None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn wrong_breakpoint() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin-left", 10, "40px", None, None, None);
        sheet.add_property("test", "margin-right", 10, "40px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_selector_with_prefix() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            1,
            "40px",
            Some(&"group-hover".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            1,
            "40px",
            Some(&"group-hover".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            2,
            "50px",
            Some(&"group-hover".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            2,
            "50px",
            Some(&"group-hover".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_theme_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-top",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        sheet.add_property("test", "margin-left", 0, "41px", None, None, None);
        sheet.add_property("test", "margin-right", 0, "41px", None, None, None);
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "51px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "51px",
            Some(&"theme-light".into()),
            None,
            None,
        );
        sheet.add_property("test", "margin-left", 0, "42px", None, None, None);
        sheet.add_property("test", "margin-right", 0, "42px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&["theme-light", "active"].into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&["theme-light", "active"].into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&["theme-light", "hover"].into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&["theme-light", "hover"].into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_print_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-top",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );

        sheet.add_property(
            "test",
            "margin-left",
            1,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            1,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-top",
            1,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            1,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"print".into()),
            None,
            None,
        );
        sheet.add_property("test", "margin-top", 0, "40px", None, None, None);
        sheet.add_property("test", "margin-bottom", 0, "40px", None, None, None);

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_screen_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&"screen".into()),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_speech_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "display",
            0,
            "none",
            Some(&"speech".into()),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_all_media_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "font-family",
            0,
            "sans-serif",
            Some(&"all".into()),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_selector_with_query() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-top",
            0,
            "40px",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width: 1024px)".to_string(),
                selector: Some("&:hover".to_string()),
            }),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Media,
                query: "(min-width: 1024px)".to_string(),
                selector: Some("&:hover".to_string()),
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_selector_with_supports() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "display",
            0,
            "grid",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "(display: grid)".to_string(),
                selector: None,
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_selector_with_container() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "padding",
            0,
            "10px",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Container,
                query: "(min-width: 768px)".to_string(),
                selector: None,
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_deserialize() {
        {
            let sheet: StyleSheet = serde_json::from_str(
                r##"{
            "properties": {
                "": {
                    "255": {
                        "0": [
                            {
                                "c": "test",
                                "p": "mx",
                                "v": "40px",
                                "s": null,
                                "b": false
                            }
                        ]
                    }
                }
            },
            "css": {},
            "theme": {
                "breakPoints": [
                    640,
                    768,
                    1024,
                    1280
                ],
                "colors": {
                    "black": "#000",
                    "white": "#fff"
                },
                "typography": {}
            }
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(sheet);
        }

        {
            let sheet: Result<StyleSheet, _> = serde_json::from_str(
                r##"{
            "properties": {
                "wrong": [
                    {
                        "c": "test",
                        "p": "mx",
                        "v": "40px",
                        "s": null,
                        "b": false
                    }
                ]
            },
            "css": [],
            "theme": {
                "breakPoints": [
                    640,
                    768,
                    1024,
                    1280
                ],
                "colors": {
                    "black": "#000",
                    "white": "#fff"
                },
                "typography": {}
            }
        }"##,
            );
            assert!(sheet.is_err());
        }
    }

    #[test]
    fn test_create_css_with_global_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();

        sheet.add_property(
            "test",
            "background-color",
            2,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
            None,
        );
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );
        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(255),
            None,
        );
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test2.tsx".to_string(),
            )),
            Some(255),
            None,
        );

        sheet.add_property(
            "test2",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Selector("&:hover".to_string())),
            Some(255),
            None,
        );

        sheet.rm_global_css("test.tsx", true);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );
        sheet.add_property(
            "test",
            "color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        sheet.rm_global_css("test.tsx", true);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            0,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );
        sheet.add_property(
            "test",
            "color",
            0,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test2.tsx".to_string(),
            )),
            Some(0),
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());

        sheet.rm_global_css("test.tsx", true);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_create_css_with_imports() {
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test2.tsx", "@devup-ui/core/css/global2.css");
            sheet.add_import("test3.tsx", "@devup-ui/core/css/global3.css");
            sheet.add_import("test4.tsx", "@devup-ui/core/css/global4.css");
            assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
        }
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test.tsx", "@devup-ui/core/css/new-global.css");
            assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
        }
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
        }
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "\"@devup-ui/core/css/global.css\" layer");
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
        }
    }

    #[test]
    fn test_get_theme_interface() {
        let sheet = StyleSheet::default();
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            ""
        );

        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("dark", color_theme);
        sheet.set_theme(theme);
        assert_debug_snapshot!(sheet.create_interface(
            "package",
            "ColorInterface",
            "TypographyInterface",
            "ThemeInterface"
        ));

        // test wrong case (backticks and special characters)
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("(primary)", "#000");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "prim``ary",
            vec![Some(Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
            ))],
        );
        sheet.set_theme(theme);
        assert_debug_snapshot!(sheet.create_interface(
            "package",
            "ColorInterface",
            "TypographyInterface",
            "ThemeInterface"
        ));

        // test nested colors - interface keys should use dots for TypeScript
        let mut sheet = StyleSheet::default();
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "gray": {
                            "100": "#f5f5f5",
                            "200": "#eee"
                        },
                        "primary": "#000",
                        "secondary.light": "#ccc"
                    }
                }
            }"##,
        )
        .unwrap();
        sheet.set_theme(theme);
        assert_debug_snapshot!(sheet.create_interface(
            "package",
            "ColorInterface",
            "TypographyInterface",
            "ThemeInterface"
        ));

        // test deep nested colors
        let mut sheet = StyleSheet::default();
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "dark": {
                        "brand": {
                            "primary": {
                                "light": "#f0f",
                                "dark": "#0f0"
                            }
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        sheet.set_theme(theme);
        assert_debug_snapshot!(sheet.create_interface(
            "package",
            "ColorInterface",
            "TypographyInterface",
            "ThemeInterface"
        ));
    }

    #[test]
    fn test_keyframes() {
        let mut sheet = StyleSheet::default();
        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        let mut from_props = BTreeSet::new();
        from_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("0"),
            selector: None,
            layer: None,
        });
        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

        let mut to_props = BTreeSet::new();
        to_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("1"),
            selector: None,
            layer: None,
        });
        keyframes.insert(
            String::from("to"),
            vec![(String::from("opacity"), String::from("1"))],
        );

        sheet.add_keyframes("fadeIn", keyframes, None);
        let past = sheet.create_css(None, false);
        assert_debug_snapshot!(past.split("*/").nth(1).unwrap());

        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut from_props = BTreeSet::new();
        from_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("0"),
            selector: None,
            layer: None,
        });
        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

        let mut to_props = BTreeSet::new();
        to_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("1"),
            selector: None,
            layer: None,
        });
        keyframes.insert(
            String::from("to"),
            vec![(String::from("opacity"), String::from("1"))],
        );

        sheet.add_keyframes("fadeIn", keyframes, None);

        let now = sheet.create_css(None, false);
        assert_debug_snapshot!(now.split("*/").nth(1).unwrap());
        assert_eq!(past, now);
    }

    #[test]
    fn test_font_face() {
        let mut sheet = StyleSheet::default();
        let mut font_face_props = BTreeMap::new();
        font_face_props.insert("font-family".to_string(), "Roboto".to_string());
        font_face_props.insert(
            "src".to_string(),
            "url('/fonts/Roboto-Regular.ttf')".to_string(),
        );
        font_face_props.insert("font-weight".to_string(), "400".to_string());

        sheet.add_font_face("test.tsx", &font_face_props);

        let css = sheet.create_css(None, false);
        assert!(css.contains("@font-face"));
        assert!(css.contains("font-family:Roboto"));
        assert!(css.contains("src:url('/fonts/Roboto-Regular.ttf')"));
        assert!(css.contains("font-weight:400"));

        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_update_styles() {
        let mut sheet = StyleSheet::default();
        sheet.update_styles(&HashSet::new(), "index.tsx", true);
        assert_debug_snapshot!(
            sheet
                .create_css(Some("index.tsx"), true)
                .split("*/")
                .nth(1)
                .unwrap()
        );

        let mut sheet = StyleSheet::default();
        let output = extract("index.tsx", "import {Box,globalCss,keyframes,Flex} from '@devup-ui/core';<Flex/>;keyframes({from:{opacity:0},to:{opacity:1}});<Box w={1} h={variable} />;globalCss`div{color:red}`;globalCss({div:{display:flex},imports:['https://test.com/a.css'],fontFaces:[{fontFamily:'Roboto',src:'url(/fonts/Roboto-Regular.ttf)'}]})", ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: std::collections::HashMap::new() }).unwrap();
        sheet.update_styles(&output.styles, "index.tsx", true);
        assert_debug_snapshot!(sheet.create_css(None, true).split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_update_styles_with_typography() {
        use extractor::extract_style::extract_style_value::ExtractStyleValue;

        let mut sheet = StyleSheet::default();
        let mut styles = HashSet::new();
        styles.insert(ExtractStyleValue::Typography("$heading".to_string()));
        let (collected, updated) = sheet.update_styles(&styles, "index.tsx", true);
        // Typography doesn't collect or update
        assert!(!collected);
        assert!(!updated);
    }

    #[test]
    fn test_global_styles_with_custom_layer() {
        let mut sheet = StyleSheet::default();
        // Add global style with layer
        sheet.add_property_with_layer(
            "*",
            "margin",
            0,
            "0",
            Some(&StyleSelector::Global(
                "*".to_string(),
                "reset.css.ts".to_string(),
            )),
            Some(0),
            None,
            Some("reset"),
        );
        sheet.add_property_with_layer(
            "*",
            "padding",
            0,
            "0",
            Some(&StyleSelector::Global(
                "*".to_string(),
                "reset.css.ts".to_string(),
            )),
            Some(0),
            None,
            Some("reset"),
        );
        // Add another layer
        sheet.add_property_with_layer(
            "body",
            "font-family",
            0,
            "sans-serif",
            Some(&StyleSelector::Global(
                "body".to_string(),
                "base.css.ts".to_string(),
            )),
            Some(0),
            None,
            Some("base"),
        );
        let css = sheet.create_css(None, false);
        // Layers are sorted alphabetically
        assert!(css.contains("@layer base,reset"));
        assert!(css.contains("@layer reset{*{margin:0;padding:0}}"));
        assert!(css.contains("@layer base{body{font-family:sans-serif}}"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_at_rules_with_breakpoints() {
        let mut sheet = StyleSheet::default();
        // Add @supports with breakpoint (level 1)
        sheet.add_property(
            "a",
            "display",
            1,
            "grid",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Supports,
                query: "(display: grid)".to_string(),
                selector: None,
            }),
            Some(0),
            None,
        );
        let css = sheet.create_css(None, false);
        assert!(css.contains("@media"));
        assert!(css.contains("@supports"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_container_with_breakpoints() {
        let mut sheet = StyleSheet::default();
        // Add @container with breakpoint (level 1)
        sheet.add_property(
            "a",
            "width",
            1,
            "100%",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Container,
                query: "(min-width: 400px)".to_string(),
                selector: None,
            }),
            Some(0),
            None,
        );
        let css = sheet.create_css(None, false);
        assert!(css.contains("@media"));
        assert!(css.contains("@container"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_theme_layer_in_css() {
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("default", color_theme);
        sheet.set_theme(theme);

        // Add some regular styles to trigger layer output
        sheet.add_property("a", "color", 0, "blue", None, Some(0), None);

        let css = sheet.create_css(None, false);
        assert!(css.contains("@layer"));
        assert!(css.contains("@layer t{"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_layer_with_breakpoints() {
        let mut sheet = StyleSheet::default();
        // Add @layer with breakpoint (level 1)
        sheet.add_property(
            "a",
            "display",
            1,
            "flex",
            Some(&StyleSelector::At {
                kind: AtRuleKind::Layer,
                query: "components".to_string(),
                selector: None,
            }),
            Some(0),
            None,
        );
        let css = sheet.create_css(None, false);
        assert!(css.contains("@media"));
        assert!(css.contains("@layer components"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }

    #[test]
    fn test_stylesheet_css_extract() {
        let css_entry = StyleSheetCss {
            css: "div{display:flex}".to_string(),
        };
        assert_eq!(css_entry.extract(), "div{display:flex}");

        let empty = StyleSheetCss { css: String::new() };
        assert_eq!(empty.extract(), "");
    }

    #[test]
    fn test_keyframes_multi_property() {
        let mut sheet = StyleSheet::default();
        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        // Multiple properties in a single keyframe step to cover the semicolon separator (line 548)
        keyframes.insert(
            String::from("from"),
            vec![
                (String::from("opacity"), String::from("0")),
                (String::from("transform"), String::from("scale(0.5)")),
            ],
        );
        keyframes.insert(
            String::from("to"),
            vec![
                (String::from("opacity"), String::from("1")),
                (String::from("transform"), String::from("scale(1)")),
            ],
        );
        sheet.add_keyframes("slideIn", keyframes, None);
        let css = sheet.create_css(None, false);
        // Verify semicolon separator between multiple properties in a keyframe step
        assert!(css.contains("opacity:0;transform:scale(0.5)"));
        assert!(css.contains("opacity:1;transform:scale(1)"));
        assert_debug_snapshot!(css.split("*/").nth(1).unwrap());
    }
}
