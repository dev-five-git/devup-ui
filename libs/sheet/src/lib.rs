pub mod theme;

use crate::theme::Theme;
use css::{
    atom_hoist::{atom_hoist_threshold, is_atom_hoist},
    file_map::canonical,
    file_routes::route_count_for_files,
    merge_selector, sheet_to_classname,
    style_selector::{AtRuleKind, StyleSelector},
    theme_tokens::set_theme_token_levels,
};
use extractor::extract_style::ExtractStyleProperty;
use extractor::extract_style::extract_static_style::ThemeTokenResolution;
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::extract_style::style_property::StyleProperty;
use regex_lite::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering::Equal;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

macro_rules! push_fmt {
    ($target:expr, $($arg:tt)*) => {{
        // `std::fmt::Write::write_fmt` on `&mut String` is infallible; discard result.
        let _ = std::fmt::Write::write_fmt($target, format_args!($($arg)*));
    }};
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
    /// CSS layer name (from vanilla-extract `layer()`)
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

impl StyleSheetProperty {
    fn write_extract(&self, css: &mut String) {
        css.push_str(&merge_selector(&self.class_name, self.selector.as_ref()));
        css.push('{');
        css.push_str(&self.property);
        css.push(':');
        css.push_str(&convert_theme_variable_value(&self.value));
        css.push('}');
    }
}

fn compile_regex(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|err| panic!("invalid built-in regex pattern `{pattern}`: {err}"))
}

static VAR_RE: LazyLock<Regex> = LazyLock::new(|| compile_regex(r"\$[\w.]+"));
static INTERFACE_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^[a-zA-Z_$][a-zA-Z0-9_$]*$"));

/// Cached header string — computed once from compile-time included package.json
static HEADER: LazyLock<String> = LazyLock::new(|| {
    format!(
        "/*! devup-ui v{version} | Apache License 2.0 | https://devup-ui.com */",
        version = include_str!("../../../bindings/devup-ui-wasm/package.json")
            .lines()
            .find(|line| line.contains("\"version\""))
            .and_then(|line| line.split(':').nth(1))
            .unwrap_or("\"unknown\"")
            .trim()
            .replace('"', ""),
    )
});

fn convert_interface_key(key: &str) -> String {
    if INTERFACE_KEY_RE.is_match(key) {
        key.to_string()
    } else {
        format!("[`{}`]", key.replace('`', "\\`"))
    }
}

fn convert_theme_variable_value(value: &str) -> Cow<'_, str> {
    if value.contains('$') {
        Cow::Owned(
            VAR_RE
                .replace_all(value, |caps: &regex_lite::Captures| {
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

type PropertyMap = BTreeMap<u8, BTreeMap<u8, FxHashSet<StyleSheetProperty>>>;
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
        BTreeMap<String, BTreeMap<String, FxHashSet<StyleSheetProperty>>>,
    >::deserialize(deserializer)?
    {
        let mut tmp_map: PropertyMap = BTreeMap::new();

        for (key, value) in value {
            let mut inner_tmp_map = BTreeMap::new();
            for (key, value) in value {
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

        // Avoid allocating an owned key when the file bucket already exists (the common case).
        let filename_key = filename.unwrap_or_default();
        if !self.properties.contains_key(filename_key) {
            self.properties
                .insert(filename_key.to_string(), BTreeMap::new());
        }
        self.properties.get_mut(filename_key).is_some_and(|bucket| {
            bucket
                .entry(style_order.unwrap_or(255))
                .or_default()
                .entry(level)
                .or_default()
                .insert(StyleSheetProperty {
                    class_name: class_name.to_string(),
                    property: property.to_string(),
                    value: value.to_string(),
                    selector: selector.cloned(),
                    layer: layer.map(ToString::to_string),
                })
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
        // @import rules are per-source-file globalCss (keyed by raw filename),
        // like `css`/`font_faces`; clear them so an @import removed from source
        // does not linger across re-extraction (HMR).
        self.imports.remove(file);
        // `file` is the RAW source filename (globalCss is per-source-file). Atoms
        // were bucketed by canonical(file) in update_styles, so global-selector
        // atom removal must read from the canonical bucket while still matching
        // the raw owner via `f == file` below.
        let property_key = if single_css {
            String::new()
        } else {
            canonical(file)
        };

        if let Some(prop_map) = self.properties.get_mut(&property_key) {
            for map in prop_map.values_mut() {
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
        }
        if self
            .properties
            .get(&property_key)
            .is_none_or(BTreeMap::is_empty)
        {
            self.properties.remove(&property_key);
        }
        true
    }

    pub fn set_theme(&mut self, theme: Theme) {
        set_theme_token_levels(
            theme.get_length_token_levels(),
            theme.get_shadow_token_levels(),
        );
        self.theme = theme;
    }

    pub fn update_styles(
        &mut self,
        styles: &FxHashSet<ExtractStyleValue>,
        filename: &str,
        single_css: bool,
    ) -> (bool, bool) {
        let mut collected = false;
        let mut updated_base_style = false;
        // Decouple class NAMING from property BUCKETING. atom_hoist uses GLOBAL
        // (prefix-less, shared-registry) names like single_css, but still keeps
        // per-file property buckets so create_css can route each atom to the
        // global chunk or a per-route chunk based on its route usage.
        let name_scope = if single_css || is_atom_hoist() {
            None
        } else {
            Some(filename)
        };
        let bucket_scope = if single_css { None } else { Some(filename) };
        for style in styles {
            match style {
                ExtractStyleValue::Static(st) => {
                    let resolved_value: Cow<'_, str> =
                        if st.theme_token_resolution() == ThemeTokenResolution::FirstValue {
                            if let Some(token) = st.value().strip_prefix('$') {
                                match st.property() {
                                    "box-shadow" => self.theme.get_default_shadow_value(token),
                                    _ => self.theme.get_default_length_value(token),
                                }
                                .map_or_else(
                                    || Cow::Borrowed(st.value()),
                                    |v| Cow::Owned(v.to_string()),
                                )
                            } else {
                                Cow::Borrowed(st.value())
                            }
                        } else {
                            Cow::Borrowed(st.value())
                        };

                    let class_name =
                        if st.theme_token_resolution() == ThemeTokenResolution::FirstValue {
                            let selector = st.selector().map(ToString::to_string);
                            sheet_to_classname(
                                st.property(),
                                st.level(),
                                Some(&resolved_value),
                                selector.as_deref(),
                                st.style_order(),
                                name_scope,
                            )
                        } else {
                            match st.extract(name_scope) {
                                StyleProperty::ClassName(cls)
                                | StyleProperty::Variable {
                                    class_name: cls, ..
                                } => cls,
                            }
                        };

                    if self.add_property_with_layer(
                        &class_name,
                        st.property(),
                        st.level(),
                        &resolved_value,
                        st.selector(),
                        st.style_order(),
                        bucket_scope,
                        st.layer(),
                    ) {
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
                    }) = style.extract(name_scope)
                        && self.add_property(
                            &class_name,
                            dy.property(),
                            dy.level(),
                            &if dy.important() {
                                format!("var({variable_name}) !important")
                            } else {
                                format!("var({variable_name})")
                            },
                            dy.selector(),
                            dy.style_order(),
                            bucket_scope,
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
                        &keyframes.extract(name_scope).to_string(),
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
                        bucket_scope,
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

    #[must_use]
    pub fn create_interface(
        &self,
        package_name: &str,
        color_interface_name: &str,
        typography_interface_name: &str,
        length_interface_name: &str,
        shadows_interface_name: &str,
        theme_interface_name: &str,
    ) -> String {
        let mut color_keys = BTreeSet::new();
        let mut typography_keys = BTreeSet::new();
        let mut length_keys = BTreeSet::new();
        let mut shadows_keys = BTreeSet::new();
        let mut theme_keys = BTreeSet::new();
        for color_theme in self.theme.colors.values() {
            color_theme.interface_keys().for_each(|key| {
                color_keys.insert(key.clone());
            });
        }
        self.theme.typography.keys().for_each(|key| {
            typography_keys.insert(key.clone());
        });
        length_keys.extend(self.theme.length.values().flat_map(|t| t.keys().cloned()));
        shadows_keys.extend(self.theme.shadows.values().flat_map(|t| t.keys().cloned()));

        self.theme.colors.keys().for_each(|key| {
            theme_keys.insert(key.clone());
        });

        if color_keys.is_empty()
            && typography_keys.is_empty()
            && length_keys.is_empty()
            && shadows_keys.is_empty()
        {
            String::new()
        } else {
            let dollar_keys = |keys: BTreeSet<String>| {
                let mut contents = String::new();
                for key in keys {
                    if !contents.is_empty() {
                        contents.push(';');
                    }
                    contents.push_str(&convert_interface_key(&format!("${key}")));
                    contents.push_str(":null");
                }
                contents
            };
            let plain_keys = |keys: BTreeSet<String>| {
                let mut contents = String::new();
                for key in keys {
                    if !contents.is_empty() {
                        contents.push(';');
                    }
                    contents.push_str(&convert_interface_key(&key));
                    contents.push_str(":null");
                }
                contents
            };
            format!(
                "import \"{}\";declare module \"{}\"{{interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}}}",
                package_name,
                package_name,
                color_interface_name,
                dollar_keys(color_keys),
                typography_interface_name,
                plain_keys(typography_keys),
                length_interface_name,
                dollar_keys(length_keys),
                shadows_interface_name,
                dollar_keys(shadows_keys),
                theme_interface_name,
                plain_keys(theme_keys)
            )
        }
    }
    fn create_style(&self, map: &BTreeMap<u8, FxHashSet<StyleSheetProperty>>) -> String {
        self.create_style_with_layers(map, &mut BTreeMap::new())
    }

    fn create_style_with_layers(
        &self,
        map: &BTreeMap<u8, FxHashSet<StyleSheetProperty>>,
        layered_styles: &mut BTreeMap<String, Vec<(String, String, String)>>, // layer -> Vec<(selector, property, value)>
    ) -> String {
        // Estimate ~64 bytes per property for pre-allocation
        let prop_count: usize = map.values().map(FxHashSet::len).sum();
        let mut current_css = String::with_capacity(prop_count * 64);
        for (level, props) in map {
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
                        map.entry((*kind, query)).or_default().push(prop);
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
                        .get(*level as usize)
                        .copied()
                        .unwrap_or_else(|| self.theme.breakpoints.last().copied().unwrap_or(0)),
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
                            selector_map.entry(selector).or_default().push(prop);
                        }
                    }
                    if let Some(break_point) = break_point {
                        push_fmt!(&mut current_css, "@media(min-width:{break_point}px){{");
                    }
                    for (selector, props) in selector_map {
                        current_css.push_str(selector);
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
                    push_fmt!(&mut current_css, "@media(min-width:{break_point}px){{");
                }
                for prop in sorted_props {
                    prop.write_extract(&mut current_css);
                }
                if break_point.is_some() {
                    current_css.push('}');
                }
            }
            for ((kind, query), props) in at_rules {
                if let Some(break_point) = break_point {
                    match kind {
                        AtRuleKind::Media => {
                            push_fmt!(
                                &mut current_css,
                                "@media(min-width:{break_point}px)and {query}{{"
                            );
                        }
                        AtRuleKind::Supports => {
                            push_fmt!(
                                &mut current_css,
                                "@media(min-width:{break_point}px){{@supports{query}{{"
                            );
                        }
                        AtRuleKind::Container => {
                            push_fmt!(
                                &mut current_css,
                                "@media(min-width:{break_point}px){{@container{query}{{"
                            );
                        }
                        AtRuleKind::Layer => {
                            push_fmt!(
                                &mut current_css,
                                "@media(min-width:{break_point}px){{@layer {query}{{"
                            );
                        }
                    }
                    for prop in props {
                        prop.write_extract(&mut current_css);
                    }
                    match kind {
                        AtRuleKind::Media => current_css.push('}'),
                        _ => current_css.push_str("}}"),
                    }
                } else {
                    push_fmt!(&mut current_css, "@{kind}");
                    if query.starts_with('(') {
                        push_fmt!(&mut current_css, "{query}{{");
                    } else {
                        push_fmt!(&mut current_css, " {query}{{");
                    }
                    for prop in props {
                        prop.write_extract(&mut current_css);
                    }
                    current_css.push('}');
                }
            }
        }
        current_css
    }

    #[inline]
    fn create_header() -> &'static str {
        &HEADER
    }

    /// Compute the set of atom class names that should be hoisted into the
    /// global stylesheet under atom-level hoisting.
    ///
    /// An atom (uniquely identified by its `class_name` under global naming) is
    /// hoisted when the number of routes that transitively use it reaches the
    /// configured threshold. Base styles (`style_order == 0`) are excluded
    /// because they are already emitted globally and shared by every chunk.
    fn compute_hoisted_atoms(&self, threshold: usize) -> FxHashSet<String> {
        // atom class_name -> set of files that reference it (order != 0)
        let mut atom_files: FxHashMap<&str, FxHashSet<&str>> = FxHashMap::default();
        for (filename, property_map) in &self.properties {
            for (style_order, level_map) in property_map {
                if *style_order == 0 {
                    continue;
                }
                for props in level_map.values() {
                    for prop in props {
                        atom_files
                            .entry(prop.class_name.as_str())
                            .or_default()
                            .insert(filename.as_str());
                    }
                }
            }
        }
        atom_files
            .into_iter()
            .filter(|(_, files)| route_count_for_files(files.iter().copied()) >= threshold)
            .map(|(class_name, _)| class_name.to_string())
            .collect()
    }

    #[must_use]
    pub fn create_css(&self, filename: Option<&str>, import_main_css: bool) -> String {
        let mut css = String::with_capacity(4096);
        css.push_str(Self::create_header());
        for import in self.imports.values().flatten() {
            if import.starts_with('"') {
                push_fmt!(&mut css, "@import {import};");
            } else {
                push_fmt!(&mut css, "@import \"{import}\";");
            }
        }

        let write_global = filename.is_none();

        // Under atom-level hoisting, decide which atoms (order != 0) live in the
        // shared global stylesheet vs. their per-route chunk.
        let hoisted_atoms: Option<FxHashSet<String>> =
            atom_hoist_threshold().map(|threshold| self.compute_hoisted_atoms(threshold));

        if write_global {
            let mut style_orders: BTreeSet<u8> = BTreeSet::new();
            let mut base_styles = BTreeMap::<u8, FxHashSet<StyleSheetProperty>>::new();
            self.properties.values().for_each(|map| {
                style_orders.extend(map.iter().filter(|(_, v)| !v.is_empty()).map(|(k, _)| *k));
                if let Some(_base_styles) = map.get(&0) {
                    _base_styles.iter().for_each(|prop| {
                        base_styles
                            .entry(*prop.0)
                            .or_default()
                            .extend(prop.1.iter().cloned());
                    });
                }
            });
            // default
            style_orders.remove(&255);
            // base style

            let theme_css = self.theme.to_css();
            let has_base = style_orders.remove(&0);
            let has_theme = !theme_css.is_empty();
            let has_orders = !style_orders.is_empty();
            if has_base || has_theme || has_orders {
                css.push_str("@layer ");
                let mut first = if has_base {
                    css.push('b');
                    false
                } else {
                    true
                };
                if has_theme {
                    if !first {
                        css.push(',');
                    }
                    css.push('t');
                    first = false;
                }
                for v in &style_orders {
                    if !first {
                        css.push(',');
                    }
                    first = false;
                    push_fmt!(&mut css, "o{v}");
                }
                css.push(';');
            }
            if !theme_css.is_empty() {
                push_fmt!(&mut css, "@layer t{{{theme_css}}}");
            }
            // One source file extracted under multiple passes (e.g. Next
            // server + client compilations) registers identical @font-face rules
            // under multiple file keys; emit each distinct rule only once.
            let mut seen_font_faces: BTreeSet<&BTreeMap<String, String>> = BTreeSet::new();
            for font_faces in self.font_faces.values() {
                for font_face in font_faces {
                    if !seen_font_faces.insert(font_face) {
                        continue;
                    }
                    css.push_str("@font-face{");
                    let mut first = true;
                    for (key, value) in font_face {
                        if !first {
                            css.push(';');
                        }
                        first = false;
                        push_fmt!(&mut css, "{key}:{value}");
                    }
                    css.push('}');
                }
            }

            // global css
            for _css in self.css.values() {
                for _css in _css {
                    css.push_str(&_css.css);
                }
            }

            // Collect layered styles while creating base CSS
            let mut layered_styles: BTreeMap<String, Vec<(String, String, String)>> =
                BTreeMap::new();
            let base_css = self.create_style_with_layers(&base_styles, &mut layered_styles);
            if !base_css.is_empty() {
                push_fmt!(&mut css, "@layer b{{{base_css}}}");
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

                    push_fmt!(&mut css, "@layer {layer_name}{{");
                    for (selector, props) in selector_map {
                        css.push_str(&selector);
                        css.push('{');
                        let mut first = true;
                        for (p, v) in props {
                            if !first {
                                css.push(';');
                            }
                            first = false;
                            push_fmt!(&mut css, "{p}:{v}");
                        }
                        css.push('}');
                    }
                    css.push('}');
                }
            }
            // Atom hoisting: emit shared (hoisted) order!=0 atoms into the global
            // stylesheet, aggregated across every file and deduplicated by atom
            // identity (class_name).
            if let Some(hoisted) = &hoisted_atoms {
                let mut aggregated: BTreeMap<u8, BTreeMap<u8, FxHashSet<StyleSheetProperty>>> =
                    BTreeMap::new();
                for property_map in self.properties.values() {
                    for (style_order, level_map) in property_map {
                        if *style_order == 0 {
                            continue;
                        }
                        for (level, props) in level_map {
                            for prop in props {
                                if hoisted.contains(&prop.class_name) {
                                    aggregated
                                        .entry(*style_order)
                                        .or_default()
                                        .entry(*level)
                                        .or_default()
                                        .insert(prop.clone());
                                }
                            }
                        }
                    }
                }
                for (style_order, map) in aggregated {
                    let current_css = self.create_style(&map);
                    if current_css.is_empty() {
                        continue;
                    }
                    if style_order == 255 {
                        css.push_str(&current_css);
                    } else {
                        push_fmt!(&mut css, "@layer o{style_order}{{{current_css}}}");
                    }
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
                push_fmt!(&mut css, "@keyframes {name}{{");
                for (key, props) in map {
                    push_fmt!(&mut css, "{key}{{");
                    let mut first = true;
                    for (k, v) in props {
                        if !first {
                            css.push(';');
                        }
                        first = false;
                        push_fmt!(&mut css, "{k}:{v}");
                    }
                    css.push('}');
                }
                css.push('}');
            }
        }

        // order
        if let Some(maps) = self.properties.get(filename.unwrap_or_default()) {
            for (style_order, map) in maps {
                if *style_order == 0 {
                    // base style was created in global css
                    continue;
                }
                // Under atom hoisting, hoisted atoms were emitted globally; the
                // per-route chunk keeps only its route-private atoms.
                let current_css = if let Some(hoisted) = &hoisted_atoms {
                    let filtered: BTreeMap<u8, FxHashSet<StyleSheetProperty>> = map
                        .iter()
                        .filter_map(|(level, props)| {
                            let kept: FxHashSet<StyleSheetProperty> = props
                                .iter()
                                .filter(|prop| !hoisted.contains(&prop.class_name))
                                .cloned()
                                .collect();
                            (!kept.is_empty()).then_some((*level, kept))
                        })
                        .collect();
                    if filtered.is_empty() {
                        continue;
                    }
                    self.create_style(&filtered)
                } else {
                    self.create_style(map)
                };

                if !current_css.is_empty() {
                    // order style 255 is user css
                    if *style_order == 255 {
                        css.push_str(&current_css);
                    } else {
                        push_fmt!(&mut css, "@layer o{style_order}{{{current_css}}}");
                    }
                }
            }
        }
        css
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use crate::theme::{ColorTheme, Typography};

    use super::*;
    use css::{class_map::reset_class_map, file_map::reset_file_map};
    use extractor::extract_style::extract_static_style::{
        ExtractStaticStyle, ThemeTokenResolution,
    };
    use extractor::{ExtractOption, extract};
    use insta::assert_debug_snapshot;

    use rstest::rstest;
    use rustc_hash::FxHashSet;
    use serial_test::serial;

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

    // Atom-level hoisting emission. Without an atom-hoist test these branches in
    // compute_hoisted_atoms / create_css were uncovered:
    //   * compute_hoisted_atoms skips style_order 0
    //   * the global hoist emission skips style_order 0
    //   * the global hoist emission skips an aggregated order whose CSS is empty
    //     (a hoisted atom that is a *layered global* prop -> no direct output)
    //   * the global hoist emission wraps a hoisted order != 255 in `@layer o{N}`
    //   * the per-route emission skips a chunk whose atoms were all hoisted away
    #[test]
    #[serial]
    fn create_css_atom_hoisting_emission() {
        use css::atom_hoist::set_atom_hoist;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};

        reset_class_map();
        reset_file_map();
        reset_file_routes();

        // a.tsx and b.tsx each own one distinct route, so an atom referenced by
        // BOTH is reached by 2 routes (>= threshold) and gets hoisted.
        let mut routes = HashMap::new();
        routes.insert("a.tsx".to_string(), HashSet::from([0u32]));
        routes.insert("b.tsx".to_string(), HashSet::from([1u32]));
        set_file_routes(routes);
        set_atom_hoist(Some(2));

        let mut sheet = StyleSheet::default();
        // Hoisted user atom (style_order 255), in both files.
        sheet.add_property("hu", "color", 0, "red", None, Some(255), Some("a.tsx"));
        sheet.add_property("hu", "color", 0, "red", None, Some(255), Some("b.tsx"));
        // Hoisted ordered atom (style_order 1) -> emitted as `@layer o1`.
        sheet.add_property("ho", "padding", 0, "1px", None, Some(1), Some("a.tsx"));
        sheet.add_property("ho", "padding", 0, "1px", None, Some(1), Some("b.tsx"));
        // Base style (style_order 0) -> exercises the style_order == 0 skips.
        sheet.add_property("hb", "margin", 0, "0", None, Some(0), Some("a.tsx"));
        // Hoisted LAYERED GLOBAL atom (style_order 2): when aggregated, its
        // create_style produces no direct CSS (it goes to the discarded layer
        // map), so that aggregated order is skipped as empty.
        let ga = StyleSelector::Global("div".to_string(), "a.tsx".to_string());
        sheet.add_property_with_layer(
            "hg",
            "border-radius",
            0,
            "9px",
            Some(&ga),
            Some(2),
            Some("a.tsx"),
            Some("lyr"),
        );
        let gb = StyleSelector::Global("div".to_string(), "b.tsx".to_string());
        sheet.add_property_with_layer(
            "hg",
            "border-radius",
            0,
            "9px",
            Some(&gb),
            Some(2),
            Some("b.tsx"),
            Some("lyr"),
        );
        // Non-hoisted responsive at-rule atom (only a.tsx): emitted in a.tsx's
        // chunk via the break-point at-rule path (level != 0 -> break_point set).
        let at = StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "(hover:hover)".to_string(),
            selector: None,
        };
        sheet.add_property(
            "atr",
            "color",
            1,
            "blue",
            Some(&at),
            Some(255),
            Some("a.tsx"),
        );

        // Global stylesheet: runs compute_hoisted_atoms + the hoist emission.
        let global_css = sheet.create_css(None, false);
        assert!(
            global_css.contains("@layer o1"),
            "hoisted order-1 atom must emit @layer o1: {global_css}"
        );

        // Per-route chunk for a.tsx: every one of its atoms was hoisted, so the
        // chunk keeps none of them (exercises the all-hoisted skip).
        let chunk_css = sheet.create_css(Some("a.tsx"), false);
        assert!(
            !chunk_css.contains("@layer o1"),
            "hoisted atoms must not duplicate into the per-route chunk: {chunk_css}"
        );
        assert!(
            !chunk_css.contains("padding:1px"),
            "hoisted padding atom must not be in the chunk: {chunk_css}"
        );
        // The responsive at-rule wrapper AND its property must both be emitted
        // (exercises the break-point at-rule path).
        assert!(
            chunk_css.contains("hover:hover"),
            "at-rule wrapper must be emitted: {chunk_css}"
        );
        assert!(
            chunk_css.contains("blue"),
            "at-rule property must be written: {chunk_css}"
        );

        set_atom_hoist(None);
        reset_file_routes();
    }

    // Under single-importer collapse, a collapsed file's globalCss atoms are
    // bucketed by canonical(file). rm_global_css(raw) must therefore clear them
    // from the CANONICAL bucket (matching the raw owner via f == file), and must
    // NOT touch the bucket-root's own global atoms.
    #[test]
    #[serial]
    fn rm_global_css_clears_collapsed_globals_from_canonical_bucket() {
        use css::file_map::{reset_canonical_map, set_canonical_map};
        reset_class_map();
        reset_file_map();
        reset_canonical_map();
        let mut m = std::collections::HashMap::new();
        m.insert("child.tsx".to_string(), "parent.tsx".to_string());
        set_canonical_map(m);

        let mut sheet = StyleSheet::default();
        // child's own globalCss: @font-face + a global selector, bucketed by
        // canonical(child) == "parent.tsx".
        sheet.add_font_face(
            "child.tsx",
            &BTreeMap::from([("font-family".to_string(), "D2Coding".to_string())]),
        );
        sheet.add_property(
            "c1",
            "border-radius",
            0,
            "10px",
            Some(&StyleSelector::Global(
                "pre".to_string(),
                "child.tsx".to_string(),
            )),
            Some(0),
            Some("parent.tsx"),
        );
        // parent's own global selector in the SAME canonical bucket.
        sheet.add_property(
            "p1",
            "border-radius",
            0,
            "5px",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "parent.tsx".to_string(),
            )),
            Some(0),
            Some("parent.tsx"),
        );

        // Clearing child's globalCss must remove ONLY child's contributions.
        sheet.rm_global_css("child.tsx", false);
        let css = sheet.create_css(None, false);
        reset_canonical_map();

        assert!(
            !css.contains("D2Coding"),
            "child @font-face not cleared: {css}"
        );
        assert!(
            !css.contains("border-radius:10px"),
            "child global atom not cleared from canonical bucket: {css}"
        );
        assert!(
            css.contains("border-radius:5px"),
            "parent global atom wrongly cleared: {css}"
        );
    }

    // A single source file extracted under multiple passes (e.g. Next server +
    // client compilations) registers the SAME @font-face under multiple file
    // keys. The emitted CSS must contain each distinct @font-face only ONCE.
    #[test]
    fn font_faces_deduplicated_across_file_keys() {
        let props = BTreeMap::from([
            ("font-family".to_string(), "Roboto".to_string()),
            ("src".to_string(), "url(/r.woff2)".to_string()),
        ]);
        let mut sheet = StyleSheet::default();
        sheet.add_font_face("a.tsx", &props);
        sheet.add_font_face("b.tsx", &props);
        let css = sheet.create_css(None, false);
        assert_eq!(
            css.matches("@font-face{").count(),
            1,
            "duplicate @font-face must be emitted once: {css}"
        );
    }

    // rm_global_css clears a file's globalCss before it is re-added on the next
    // extraction (HMR). It must also drop the file's @import rules, otherwise an
    // @import removed from source lingers until restart.
    #[test]
    fn rm_global_css_clears_imports() {
        let mut sheet = StyleSheet::default();
        sheet.add_import("a.tsx", "\"https://example.com/stale.css\"");
        assert!(sheet.create_css(None, false).contains("stale.css"));
        sheet.rm_global_css("a.tsx", false);
        let css = sheet.create_css(None, false);
        assert!(
            !css.contains("stale.css"),
            "rm_global_css must clear stale @import: {css}"
        );
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
                "LengthInterface",
                "ShadowsInterface",
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
            "LengthInterface",
            "ShadowsInterface",
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
            "LengthInterface",
            "ShadowsInterface",
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
            "LengthInterface",
            "ShadowsInterface",
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
            "LengthInterface",
            "ShadowsInterface",
            "ThemeInterface"
        ));

        // Multiple typography keys + multiple color themes exercise the
        // `plain_keys` semicolon separator (joins 2+ entries).
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut light_theme = ColorTheme::default();
        light_theme.add_color("primary", "#000");
        let mut dark_theme = ColorTheme::default();
        dark_theme.add_color("primary", "#fff");
        theme.add_color_theme("default", light_theme);
        theme.add_color_theme("dark", dark_theme);
        let make_typography = || {
            Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
            )
        };
        theme.add_typography("heading", vec![Some(make_typography())]);
        theme.add_typography("body", vec![Some(make_typography())]);
        sheet.set_theme(theme);
        assert_debug_snapshot!(sheet.create_interface(
            "package",
            "ColorInterface",
            "TypographyInterface",
            "LengthInterface",
            "ShadowsInterface",
            "ThemeInterface"
        ));
    }

    #[test]
    fn test_keyframes() {
        let mut sheet = StyleSheet::default();
        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

        keyframes.insert(
            String::from("to"),
            vec![(String::from("opacity"), String::from("1"))],
        );

        sheet.add_keyframes("fadeIn", keyframes, None);
        let past = sheet.create_css(None, false);
        assert_debug_snapshot!(past.split("*/").nth(1).unwrap());

        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

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
    #[serial]
    fn test_update_styles() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        sheet.update_styles(&FxHashSet::default(), "index.tsx", true);
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
        let mut styles = FxHashSet::default();
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
    fn test_stylesheet_css_struct() {
        let css_entry = StyleSheetCss {
            css: "div{display:flex}".to_string(),
        };
        assert_eq!(css_entry.css, "div{display:flex}");

        let empty = StyleSheetCss { css: String::new() };
        assert_eq!(empty.css, "");
    }

    #[test]
    fn test_stylesheet_property_ord_no_selectors() {
        // Both sides without selectors: branches on property then value.
        let make = |property: &str, value: &str| StyleSheetProperty {
            class_name: "a".to_string(),
            property: property.to_string(),
            value: value.to_string(),
            selector: None,
            layer: None,
        };
        assert_eq!(make("color", "red").cmp(&make("color", "red")), Equal);
        assert!(make("color", "red") < make("color", "white"));
        assert!(make("color", "red") < make("display", "block"));
        assert!(make("display", "block") > make("color", "white"));
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

    #[test]
    #[serial]
    fn test_first_value_theme_token_resolution_uses_base_value_only() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "containerX": ["1px", null, "2px"]
                    }
                }
            }"#,
        )
        .unwrap();
        sheet.set_theme(theme);

        let mut styles = FxHashSet::default();
        styles.insert(ExtractStyleValue::Static(
            ExtractStaticStyle::new("width", "$containerX", 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ));

        let (collected, _) = sheet.update_styles(&styles, "test.tsx", true);
        assert!(collected);

        let css = sheet.create_css(None, false);
        assert!(css.contains("width:1px"));
        assert!(!css.contains("width:2px"));
    }

    #[test]
    #[serial]
    fn test_first_value_without_dollar_prefix_uses_raw_value() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();

        let mut styles = FxHashSet::default();
        // FirstValue resolution but value has no $ prefix — should use the raw value as-is
        styles.insert(ExtractStyleValue::Static(
            ExtractStaticStyle::new("width", "100px", 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ));

        let (collected, _) = sheet.update_styles(&styles, "test.tsx", true);
        assert!(collected);

        let css = sheet.create_css(None, false);
        assert!(css.contains("width:100px"));
    }

    #[test]
    #[serial]
    fn test_first_value_box_shadow_resolves_shadow_token() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        let theme: Theme = serde_json::from_str(
            r#"{
                "shadow": {
                    "default": {
                        "card": ["0 1px 2px #0003", null, "0 4px 8px #0003"]
                    }
                }
            }"#,
        )
        .unwrap();
        sheet.set_theme(theme);

        let mut styles = FxHashSet::default();
        styles.insert(ExtractStyleValue::Static(
            ExtractStaticStyle::new("box-shadow", "$card", 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ));

        let (collected, _) = sheet.update_styles(&styles, "test.tsx", true);
        assert!(collected);

        let css = sheet.create_css(None, false);
        assert!(css.contains("box-shadow:0 1px 2px #0003"));
    }

    #[test]
    fn test_important_in_css_via_add_property() {
        // Verify that !important in the value is preserved in the final CSS output
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "var(--a) !important",
            None,
            None,
            None,
        );
        let css = sheet.create_css(None, false);
        let css_body = css.split("*/").nth(1).unwrap();
        assert!(
            css_body.contains("background:var(--a) !important"),
            "CSS should contain !important. Got: {css_body}",
        );
    }

    #[test]
    #[serial]
    fn test_dynamic_style_important_full_pipeline() {
        // Full pipeline: extract JSX with `${color} !important` → sheet → CSS
        // Verifies !important ends up on the CSS property, not in the style attribute
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();

        let output = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
const color = "red";
<Box bg={`${color} !important`} />
"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: std::collections::HashMap::new(),
            },
        )
        .unwrap();

        let (collected, _) = sheet.update_styles(&output.styles, "test.tsx", true);
        assert!(collected);

        let css = sheet.create_css(None, false);
        let css_body = css.split("*/").nth(1).unwrap();
        assert!(
            css_body.contains("!important"),
            "CSS output should contain !important for dynamic styles. Got: {css_body}",
        );
        // Verify the code has clean style value (no !important in the variable)
        assert!(
            !output.code.contains("!important"),
            "Generated code should NOT contain !important in style vars. Got: {}",
            output.code,
        );
    }
}
