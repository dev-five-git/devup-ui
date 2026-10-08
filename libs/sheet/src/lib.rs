pub mod cache_snapshot;
pub mod name_registry;
#[cfg(test)]
mod name_registry_tests;
mod style_claims;
pub mod theme;

#[cfg(test)]
mod atom_identity_tests;
#[cfg(test)]
mod theme_declaration_coverage_tests;

#[cfg(test)]
mod owner_reset_tests;

#[cfg(test)]
mod sheet_test_code;

use crate::theme::Theme;
use css::{
    at_rule::{MediaCombination, combine_media_queries, query_order},
    atom_hoist::{atom_plan, freeze_atom_plan, is_atom_hoist, is_hoisted_bucket},
    file_map::canonical,
    get_custom_shorthand_names,
    style_selector::{
        AtRule, AtRuleKind, StyleSelector, get_selector_order, global_selector_order, write_at_rule,
    },
    theme_tokens::{set_theme_token_levels, set_theme_token_values, set_typography_keys},
    utils::compile_regex,
    write_merge_selector,
};
use extractor::extract_style::ExtractStyleProperty;
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::extract_style::style_property::StyleProperty;
use regex_lite::Regex;
use rustc_hash::FxHashSet;
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

const TYPOGRAPHY_LAYER: &str = "t";

/// Plain rules first, then pseudo selectors in `SELECTOR_ORDER`.
fn selector_group(selector: Option<&str>) -> (u8, u8) {
    selector.map_or((0, 0), |selector| (1, get_selector_order(selector)))
}

/// Typography preset declarations come first so a declaration written directly
/// on the same selector wins, and each preset stays contiguous so its rules merge.
fn prop_cmp(a: &StyleSheetProperty, b: &StyleSheetProperty) -> std::cmp::Ordering {
    b.typography.cmp(&a.typography).then_with(|| {
        if a.typography {
            a.class_name
                .cmp(&b.class_name)
                .then_with(|| a.property.cmp(&b.property))
                .then_with(|| a.value.cmp(&b.value))
        } else {
            a.property
                .cmp(&b.property)
                .then_with(|| a.value.cmp(&b.value))
                .then_with(|| a.class_name.cmp(&b.class_name))
        }
    })
}

/// (selector group, level, selector)
type RuleOrder<'a> = ((u8, u8), u8, &'a str);
/// (enclosing at-rules outermost first, selector group, level, selector)
type AtRuleOrder<'a> = (Vec<(u8, (u8, i64), &'a str)>, (u8, u8), u8, &'a str);

/// The blocks a rule sits in: its breakpoint level and at-rule chain.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Wrapper<'a> {
    level: u8,
    at_rule: Option<(&'a [AtRule], AtRuleKind, &'a str)>,
}

/// Global selectors without a pseudo part first, then by `SELECTOR_ORDER`.
fn global_selector_group(selector: &str) -> (bool, u8) {
    selector.find(':').map_or((false, 0), |i| {
        (true, global_selector_order(&selector[i..]))
    })
}

type GlobalProp<'a> = (u8, &'a str, &'a StyleSheetProperty);

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
    /// Declaration expanded from a conditional `typography` preset
    #[serde(rename = "t", default, skip_serializing_if = "std::ops::Not::not")]
    pub typography: bool,
    /// Placement decided before extraction, independent of later configuration.
    #[serde(rename = "h", default, skip_serializing_if = "std::ops::Not::not")]
    pub hoisted: bool,
    /// Generated variable reset, merged into the owner's plain base rule.
    #[serde(rename = "r", default, skip_serializing_if = "std::ops::Not::not")]
    pub owner_reset: bool,
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
    fn emission_identity(&self) -> Self {
        let mut emitted = self.clone();
        match emitted.selector.as_mut() {
            Some(StyleSelector::Global(_, owner)) => owner.clear(),
            Some(StyleSelector::At { file, .. }) => *file = None,
            Some(StyleSelector::Selector(_)) | None => {}
        }
        emitted
    }

    fn same_rule(&self, other: &Self) -> bool {
        self.class_name == other.class_name && self.selector == other.selector
    }

    fn write_declaration(&self, css: &mut String) {
        css.push_str(&self.property);
        css.push(':');
        css.push_str(&convert_theme_variable_value(&self.value));
    }
}

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

fn convert_interface_key(key: &str) -> Cow<'_, str> {
    if INTERFACE_KEY_RE.is_match(key) {
        Cow::Borrowed(key)
    } else {
        // Only allocate the `key.replace('`', "\\`")` intermediate when `key` actually holds a
        // backtick (the common non-identifier key — e.g. `(primary)`, `a.b` — has none). Escaping
        // a backtick-free key is a no-op, so borrowing it is byte-identical and drops one heap
        // allocation per backtick-free non-identifier key.
        let escaped = if key.contains('`') {
            Cow::Owned(key.replace('`', "\\`"))
        } else {
            Cow::Borrowed(key)
        };
        Cow::Owned(format!("[`{escaped}`]"))
    }
}

fn convert_theme_variable_value(value: &str) -> Cow<'_, str> {
    css::content_value::emitted(value)
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize, Ord, PartialOrd)]
pub struct StyleSheetCss {
    pub css: String,
}

type PropertyMap = BTreeMap<u8, BTreeMap<u8, FxHashSet<StyleSheetProperty>>>;
type KeyframesMap = BTreeMap<String, BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>>;
/// layer name -> Vec<(selector, property, value)> collected for `@layer` output.
type LayeredStyles = BTreeMap<String, String>;

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
#[derive(Default, Serialize)]
pub struct StyleSheet {
    #[serde(skip)]
    pub cache_restore: cache_snapshot::CacheRestore,
    #[serde(default)]
    pub names: name_registry::NameRegistry,
    #[serde(default)]
    pub atom_plan: Option<BTreeSet<String>>,
    #[serde(default)]
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
    /// Numbers of the original sources, read back so dynamic styles keep the
    /// variables they were named by; written by `export_snapshot`.
    #[serde(rename = "sourceIds", default, skip_serializing)]
    pub source_ids: BTreeMap<String, u32>,
    #[serde(skip)]
    pub theme: Theme,
}

impl StyleSheet {
    /// Borrow the sheet with its naming generation for build-cache fingerprints.
    pub fn export_snapshot(&self) -> impl Serialize + '_ {
        cache_snapshot::export(self)
    }

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
        self.insert_property(
            level,
            style_order,
            filename,
            StyleSheetProperty {
                class_name: class_name.to_string(),
                property: property.to_string(),
                value: value.to_string(),
                selector: selector.cloned(),
                layer: layer.map(ToString::to_string),
                typography: false,
                hoisted: false,
                owner_reset: false,
            },
        )
    }

    fn insert_property(
        &mut self,
        level: u8,
        style_order: Option<u8>,
        filename: Option<&str>,
        mut prop: StyleSheetProperty,
    ) -> bool {
        freeze_atom_plan();
        if self.atom_plan.is_none() {
            self.atom_plan = atom_plan();
        }
        prop.hoisted =
            is_atom_hoist() && style_order != Some(0) && filename.is_some_and(is_hoisted_bucket);
        // register global css file for cache
        if let Some(
            StyleSelector::Global(_, file)
            | StyleSelector::At {
                file: Some(file), ..
            },
        ) = &prop.selector
        {
            // Probe with the borrowed `&str` first so the owned `String` is only
            // allocated on first registration, not on repeat (HMR/multi-property) calls.
            // Matches the borrow-probe-first pattern in add_import/add_font_face/add_css.
            if !self.global_css_files.contains(file.as_str()) {
                self.global_css_files.insert(file.clone());
            }
        }

        // Look the file bucket up once (one probe on the common existing-file path) and only
        // allocate the owned key `String` when the bucket has to be created for the first time.
        let filename_key = filename.unwrap_or_default();
        let bucket = match self.properties.get_mut(filename_key) {
            Some(bucket) => bucket,
            None => self.properties.entry(filename_key.to_string()).or_default(),
        };
        bucket
            .entry(style_order.unwrap_or(255))
            .or_default()
            .entry(level)
            .or_default()
            .insert(prop)
    }

    pub fn add_import(&mut self, file: &str, import: &str) {
        // Probe with the borrowed `&str` first so the owned `String` is only
        // allocated on first registration, not on repeat (HMR/multi-property) calls.
        if !self.global_css_files.contains(file) {
            self.global_css_files.insert(file.to_string());
        }
        let bucket = match self.imports.get_mut(file) {
            Some(bucket) => bucket,
            None => self.imports.entry(file.to_string()).or_default(),
        };
        bucket.insert(import.to_string());
    }

    pub fn add_font_face(&mut self, file: &str, properties: &BTreeMap<String, String>) {
        // Probe with the borrowed `&str` first so the owned `String` is only
        // allocated on first registration, not on repeat (HMR/multi-property) calls.
        if !self.global_css_files.contains(file) {
            self.global_css_files.insert(file.to_string());
        }
        let bucket = match self.font_faces.get_mut(file) {
            Some(bucket) => bucket,
            None => self.font_faces.entry(file.to_string()).or_default(),
        };
        bucket.insert(properties.clone());
    }

    pub fn add_css(&mut self, file: &str, css: &str) -> bool {
        // Probe with the borrowed `&str` first so the owned `String` is only
        // allocated on first registration, not on repeat (HMR/multi-property) calls.
        if !self.global_css_files.contains(file) {
            self.global_css_files.insert(file.to_string());
        }
        let bucket = match self.css.get_mut(file) {
            Some(bucket) => bucket,
            None => self.css.entry(file.to_string()).or_default(),
        };
        bucket.insert(StyleSheetCss {
            css: css.to_string(),
        })
    }

    pub fn add_keyframes(
        &mut self,
        name: &str,
        keyframes: BTreeMap<String, Vec<(String, String)>>,
        filename: Option<&str>,
    ) -> bool {
        let filename_key = filename.unwrap_or_default();
        // Probe the outer filename key with borrowed &str first; allocate owned String only on miss.
        let inner_map = match self.keyframes.get_mut(filename_key) {
            Some(inner_map) => inner_map,
            None => self.keyframes.entry(filename_key.to_string()).or_default(),
        };
        // Probe the inner name key with borrowed &str first; allocate owned String only on miss.
        let map = match inner_map.get_mut(name) {
            Some(map) => map,
            None => inner_map.entry(name.to_string()).or_default(),
        };
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

        let bucket_empty = if let Some(prop_map) = self.properties.get_mut(&property_key) {
            for map in prop_map.values_mut() {
                for props in map.values_mut() {
                    props.retain(|prop| match prop.selector.as_ref() {
                        Some(
                            StyleSelector::Global(_, f) | StyleSelector::At { file: Some(f), .. },
                        ) => f != file,
                        _ => true,
                    });
                }
                // remove empty map
                if map.iter().all(|(_, v)| v.is_empty()) {
                    map.clear();
                }
            }
            prop_map.is_empty()
        } else {
            // Bucket absent entirely: treat as empty so it is (already) not present.
            true
        };
        if bucket_empty {
            self.properties.remove(&property_key);
        }
        true
    }

    pub fn set_theme(&mut self, theme: Theme) {
        let length = theme.get_length_token_levels();
        let shadow = theme.get_shadow_token_levels();
        let first_length = length
            .keys()
            .filter_map(|token| {
                theme
                    .get_default_length_value(token)
                    .map(|value| (token.clone(), value.to_string()))
            })
            .collect();
        let first_shadow = shadow
            .keys()
            .filter_map(|token| {
                theme
                    .get_default_shadow_value(token)
                    .map(|value| (token.clone(), value.to_string()))
            })
            .collect();
        set_theme_token_levels(length, shadow);
        set_theme_token_values(first_length, first_shadow);
        set_typography_keys(theme.typography.keys().cloned().collect());
        css::content_typography::set(
            theme
                .typography
                .keys()
                .map(|preset| {
                    (
                        preset.clone(),
                        theme
                            .typography_declarations(preset, 0)
                            .into_iter()
                            .map(|(level, property, value)| (level, property.to_string(), value))
                            .collect(),
                    )
                })
                .collect(),
        );
        self.theme = theme;
    }

    pub fn update_styles(
        &mut self,
        styles: &FxHashSet<ExtractStyleValue>,
        filename: &str,
        single_css: bool,
    ) -> Result<(bool, bool), name_registry::NameError> {
        let claims = self.preflight_styles(styles, filename, single_css)?;
        self.names.extend(claims);
        let mut collected = false;
        let mut updated_base_style = false;
        freeze_atom_plan();
        if self.atom_plan.is_none() {
            self.atom_plan = atom_plan();
        }
        let name_scope = if single_css { None } else { Some(filename) };
        let bucket_scope = if single_css { None } else { Some(filename) };
        let atom_mode = is_atom_hoist();
        let shared_bucket = single_css || (atom_mode && is_hoisted_bucket(filename));
        let updates_shared = |order: Option<u8>| {
            order == Some(0)
                || (atom_mode && (shared_bucket || order.is_some_and(|order| order != 255)))
        };
        // Names are handed out in the order styles are first seen, so the
        // set is walked in a fixed order, not the hash order.
        let mut ordered: Vec<&ExtractStyleValue> = styles.iter().collect();
        ordered.sort_unstable();
        for style in ordered {
            match style {
                // A conditional `typography` preset: its class is the atom, and the
                // preset's declarations are emitted under the atom's selector.
                ExtractStyleValue::Static(st) if st.property() == "typography" => {
                    let (StyleProperty::ClassName(class_name)
                    | StyleProperty::Variable { class_name, .. }) = st.extract(name_scope);
                    for (level, property, value) in
                        css::content_typography::declarations(st.value(), st.level())
                    {
                        if self.insert_property(
                            level,
                            st.style_order(),
                            bucket_scope,
                            StyleSheetProperty {
                                class_name: class_name.clone(),
                                property,
                                value,
                                selector: st.selector().cloned(),
                                // Under the declarations written directly, like the
                                // theme's typography classes, even when those only
                                // apply on a condition
                                layer: Some(st.layer().map_or_else(
                                    || TYPOGRAPHY_LAYER.to_string(),
                                    |layer| format!("{layer}.{TYPOGRAPHY_LAYER}"),
                                )),
                                typography: true,
                                hoisted: false,
                                owner_reset: false,
                            },
                        ) {
                            collected = true;
                            if updates_shared(st.style_order()) {
                                updated_base_style = true;
                            }
                        }
                    }
                }
                ExtractStyleValue::Static(st) => {
                    let resolved_value = st.effective_value();
                    let class_name = match st.extract(name_scope) {
                        StyleProperty::ClassName(cls)
                        | StyleProperty::Variable {
                            class_name: cls, ..
                        } => cls,
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
                        if updates_shared(st.style_order()) {
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
                        && {
                            // Build `var(<name>)` / `var(<name>) !important` without the
                            // `format!` `Arguments` machinery + its grow path: presize once and
                            // `push_str`. Byte-identical to the two `format!` calls it replaces.
                            let important = dy.important();
                            let mut dynamic_value = String::with_capacity(
                                "var(".len()
                                    + variable_name.len()
                                    + 1
                                    + if important { " !important".len() } else { 0 },
                            );
                            dynamic_value.push_str("var(");
                            dynamic_value.push_str(&variable_name);
                            dynamic_value.push(')');
                            if important {
                                dynamic_value.push_str(" !important");
                            }
                            self.add_property_with_layer(
                                &class_name,
                                dy.property(),
                                dy.level(),
                                &dynamic_value,
                                dy.selector(),
                                dy.style_order(),
                                bucket_scope,
                                dy.layer(),
                            )
                        }
                    {
                        collected = true;
                        if updates_shared(dy.style_order()) {
                            updated_base_style = true;
                        }
                    }
                    // Custom properties inherit: an element that sets nothing would
                    // read its ancestor's value, so the element's own class resets
                    // the variable, which a value set inline then overrides.
                    if let Some(StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    }) = style.extract(name_scope)
                        && self.insert_property(
                            0,
                            dy.style_order(),
                            bucket_scope,
                            StyleSheetProperty {
                                class_name,
                                property: variable_name,
                                value: "initial".to_string(),
                                selector: None,
                                layer: None,
                                typography: false,
                                hoisted: false,
                                owner_reset: true,
                            },
                        )
                    {
                        collected = true;
                        if updates_shared(dy.style_order()) {
                            updated_base_style = true;
                        }
                    }
                }

                ExtractStyleValue::Keyframes(keyframes) => {
                    let name = match keyframes.extract(name_scope) {
                        StyleProperty::ClassName(cls)
                        | StyleProperty::Variable {
                            class_name: cls, ..
                        } => cls,
                    };
                    if self.add_keyframes(
                        &name,
                        keyframes.effective_steps().into_iter().collect(),
                        bucket_scope,
                    ) {
                        collected = true;
                        updated_base_style |= atom_mode && single_css;
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
                    let added = self
                        .imports
                        .get(st.file.as_str())
                        .is_none_or(|imports| !imports.contains(st.url.as_str()));
                    self.add_import(&st.file, &st.url);
                    updated_base_style |= atom_mode && added;
                }
                ExtractStyleValue::FontFace(font) => {
                    let added = self
                        .font_faces
                        .get(font.file.as_str())
                        .is_none_or(|fonts| !fonts.contains(&font.properties));
                    self.add_font_face(&font.file, &font.properties);
                    updated_base_style |= atom_mode && added;
                }
            }
        }
        Ok((collected, updated_base_style))
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
        // Collect a `BTreeSet<&str>` that borrows each key straight from the
        // theme via `String::as_str` — no per-key `String` clone; the set only
        // owns its tree nodes, not the key bytes. Unifies the four
        // near-identical key-collection blocks (color/typography/length/shadow
        // + the theme-variant set) so which keys go into which set stays
        // byte-identical while removing the copy-paste.
        fn collect_keys<'k>(keys: &mut dyn Iterator<Item = &'k String>) -> BTreeSet<&'k str> {
            keys.map(String::as_str).collect::<BTreeSet<&'k str>>()
        }

        let color_keys = collect_keys(
            &mut self
                .theme
                .colors
                .values()
                .flat_map(theme::ColorTheme::interface_keys),
        );
        let typography_keys = collect_keys(&mut self.theme.typography.keys());
        let length_keys = collect_keys(&mut self.theme.length.values().flat_map(|t| t.keys()));
        let shadows_keys = collect_keys(&mut self.theme.shadows.values().flat_map(|t| t.keys()));
        let shorthand_keys = get_custom_shorthand_names();

        if color_keys.is_empty()
            && typography_keys.is_empty()
            && length_keys.is_empty()
            && shadows_keys.is_empty()
            && shorthand_keys.is_empty()
        {
            String::new()
        } else {
            // `theme_keys` borrows `colors.keys()` into a `BTreeSet<&str>` — no
            // key clones, but building it still costs a tree allocation plus one
            // node insert per key. It is only consumed by the
            // `emit_keys(theme_keys, false)` below, so collect it lazily here: the
            // empty-theme early-return above never builds this set just to drop it.
            let theme_keys = collect_keys(&mut self.theme.colors.keys());
            // Single emitter parameterized by whether each key is prefixed with
            // `$` (color/length/shadow interfaces) or not (typography/theme).
            // The `$` path reuses one scratch `String` across keys; the plain
            // path feeds the key straight to `convert_interface_key` so its
            // allocation profile stays identical to the former `plain_keys`.
            let emit_keys = |keys: BTreeSet<&str>, dollar: bool| {
                // Presize the buffer from the known key count: each key emits its
                // (possibly `$`-prefixed) converted name plus `:null` and a `;`
                // separator. `keys.len() * 12` is a cheap lower-bound estimate that
                // removes the 0→8→16→… grow-realloc chain; output stays byte-identical.
                let mut contents = String::with_capacity(keys.len() * 12);
                // The `$`-prefix scratch is reused across every key. Presize it
                // once to `1 + longest key len` so the very first `push_str(key)`
                // never triggers the 1→N grow-realloc; thereafter it is reused with
                // no further reallocs. The `'$'` prefix is invariant, so seed it once
                // and only rewrite the suffix each key (`truncate(1)` keeps the `$`,
                // skipping a re-push per key); output stays byte-identical.
                let dollar_cap = 1 + keys.iter().map(|k| k.len()).max().unwrap_or(0);
                let mut dollar_key = String::with_capacity(dollar_cap);
                dollar_key.push('$');
                for key in keys {
                    if !contents.is_empty() {
                        contents.push(';');
                    }
                    if dollar {
                        dollar_key.truncate(1);
                        dollar_key.push_str(key);
                        contents.push_str(&convert_interface_key(&dollar_key));
                    } else {
                        contents.push_str(&convert_interface_key(key));
                    }
                    contents.push_str(":null");
                }
                contents
            };
            let shorthand_interface = if shorthand_keys.is_empty() {
                String::new()
            } else {
                format!(
                    "interface DevupCustomShorthands{{{}}}",
                    shorthand_keys
                        .iter()
                        .map(|key| format!("{}?:DevupProps[\"w\"]", convert_interface_key(key)))
                        .collect::<Vec<_>>()
                        .join(";")
                )
            };
            let shorthand_import = if shorthand_keys.is_empty() {
                String::new()
            } else {
                format!("import type{{DevupProps}}from\"{package_name}\";")
            };
            format!(
                "import \"{}\";{}declare module \"{}\"{{interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}{}interface {}{{{}}}}}",
                package_name,
                shorthand_import,
                package_name,
                color_interface_name,
                emit_keys(color_keys, true),
                typography_interface_name,
                emit_keys(typography_keys, false),
                length_interface_name,
                emit_keys(length_keys, true),
                shadows_interface_name,
                emit_keys(shadows_keys, true),
                shorthand_interface,
                theme_interface_name,
                emit_keys(theme_keys, false)
            )
        }
    }
    fn create_style(&self, map: &BTreeMap<u8, FxHashSet<StyleSheetProperty>>) -> String {
        // Callers here discard layered output, so pass `None` to skip the
        // throwaway `BTreeMap` allocation (and the per-prop `String` clones it
        // would collect) entirely.
        self.create_style_with_layers(map, None)
    }

    // Generic over the per-level set element `P: Borrow<StyleSheetProperty>` so this one
    // implementation serves BOTH the owned `FxHashSet<StyleSheetProperty>` callers and the
    // borrowed `FxHashSet<&StyleSheetProperty>` base-style path in `create_css` — the latter
    // aggregates cross-file base props by reference (deduped by value in the set) instead of
    // deep-cloning every `StyleSheetProperty` (3 owned `String`s + `Option<StyleSelector>`).
    // Each `prop` here is a `&P`; `.borrow()` yields the `&StyleSheetProperty` all downstream
    // buckets already hold. Output is byte-identical.
    /// Layered properties are written with the same rules as the others, each
    /// layer on its own: into `layered_styles` when given, else as trailing
    /// `@layer` blocks.
    fn create_style_with_layers<P: std::borrow::Borrow<StyleSheetProperty>>(
        &self,
        map: &BTreeMap<u8, FxHashSet<P>>,
        mut layered_styles: Option<&mut LayeredStyles>,
    ) -> String {
        let mut css = self.create_layer_style(map, None);
        let layers: BTreeSet<&str> = map
            .values()
            .flatten()
            .filter_map(|prop| prop.borrow().layer.as_deref())
            .collect();
        for layer in layers {
            let layer_css = self.create_layer_style(map, Some(layer));
            match layered_styles.as_deref_mut() {
                // Typography stays nested in the origin it was written in, below
                // what that origin declares directly
                Some(layered) if layer != TYPOGRAPHY_LAYER => layered
                    .entry(layer.to_string())
                    .or_default()
                    .push_str(&layer_css),
                _ => push_fmt!(&mut css, "@layer {layer}{{{layer_css}}}"),
            }
        }
        css
    }

    /// The properties of `map` in `layer` (`None`: in no layer)
    fn create_layer_style<P: std::borrow::Borrow<StyleSheetProperty>>(
        &self,
        map: &BTreeMap<u8, FxHashSet<P>>,
        layer: Option<&str>,
    ) -> String {
        // Estimate ~64 bytes per property for pre-allocation
        let prop_count: usize = map.values().map(FxHashSet::len).sum();
        let mut current_css = String::with_capacity(prop_count * 64);
        let mut class_rules: Vec<(RuleOrder<'_>, &StyleSheetProperty)> =
            Vec::with_capacity(prop_count);
        let mut at_rules: Vec<(AtRuleOrder<'_>, Wrapper<'_>, &StyleSheetProperty)> = Vec::new();
        let mut global_props: Vec<GlobalProp<'_>> = Vec::new();
        for (level, props) in map {
            for prop in props {
                let prop: &StyleSheetProperty = prop.borrow();
                if prop.layer.as_deref() != layer {
                    continue;
                }
                match &prop.selector {
                    Some(StyleSelector::Global(selector, _)) => {
                        global_props.push((*level, selector.as_str(), prop));
                    }
                    Some(StyleSelector::At {
                        kind,
                        query,
                        selector,
                        outer,
                        ..
                    }) => {
                        let chain = outer
                            .iter()
                            .map(|rule| (rule.kind, rule.query.as_str()))
                            .chain(std::iter::once((*kind, query.as_str())))
                            .map(|(kind, query)| (kind as u8, query_order(kind, query), query))
                            .collect();
                        let selector = selector.as_deref();
                        at_rules.push((
                            (
                                chain,
                                selector_group(selector),
                                *level,
                                selector.unwrap_or(""),
                            ),
                            Wrapper {
                                level: *level,
                                at_rule: Some((outer.as_slice(), *kind, query.as_str())),
                            },
                            prop,
                        ));
                    }
                    Some(StyleSelector::Selector(selector)) => class_rules.push((
                        (selector_group(Some(selector)), *level, selector.as_str()),
                        prop,
                    )),
                    None => class_rules.push(((selector_group(None), *level, ""), prop)),
                }
            }
        }
        if !global_props.is_empty() {
            self.write_global_props(&mut current_css, global_props);
        }

        // Selector group (plain, then `SELECTOR_ORDER`) sorts before the breakpoint
        // level, so `:active` still follows a `:hover` set at a wider breakpoint while
        // one selector's responsive values keep ascending. At-rules follow every plain
        // rule so a condition beats the breakpoint values it overrides; among
        // themselves they order by condition, then selector group, then level.
        class_rules.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| prop_cmp(a.1, b.1)));
        at_rules.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| prop_cmp(a.2, b.2)));

        let class_rules = if class_rules.iter().any(|rule| rule.1.owner_reset) {
            let mut resets: BTreeMap<&str, Vec<_>> = BTreeMap::new();
            class_rules.retain(|rule| {
                if rule.1.owner_reset {
                    resets.entry(&rule.1.class_name).or_default().push(*rule);
                    false
                } else {
                    true
                }
            });
            let mut merged = Vec::with_capacity(class_rules.len());
            for rule in class_rules {
                if rule.0.1 == 0
                    && rule.1.selector.is_none()
                    && let Some(owner_resets) = resets.remove(rule.1.class_name.as_str())
                {
                    merged.extend(owner_resets);
                }
                merged.push(rule);
            }
            resets.into_values().flatten().chain(merged).collect()
        } else {
            class_rules
        };

        let mut open: Option<(Wrapper<'_>, Option<usize>)> = None;
        let mut open_rule: Option<&StyleSheetProperty> = None;
        for (wrapper, prop) in class_rules
            .iter()
            .map(|((_, level, _), prop)| {
                (
                    Wrapper {
                        level: *level,
                        at_rule: None,
                    },
                    *prop,
                )
            })
            .chain(at_rules.iter().map(|(_, wrapper, prop)| (*wrapper, *prop)))
        {
            if open.as_ref().is_none_or(|(current, _)| *current != wrapper) {
                if open_rule.take().is_some() {
                    current_css.push('}');
                }
                if let Some((_, Some(depth))) = open {
                    current_css.extend(std::iter::repeat_n('}', depth));
                }
                let depth = self.open_wrapper(&mut current_css, wrapper);
                open = Some((wrapper, depth));
            }
            if let Some((_, Some(_))) = open {
                if open_rule.is_some_and(|rule| rule.same_rule(prop)) {
                    current_css.push(';');
                } else {
                    if open_rule.is_some() {
                        current_css.push('}');
                    }
                    write_merge_selector(
                        &mut current_css,
                        &prop.class_name,
                        prop.selector.as_ref(),
                    );
                    current_css.push('{');
                    open_rule = Some(prop);
                }
                prop.write_declaration(&mut current_css);
            }
        }
        if open_rule.is_some() {
            current_css.push('}');
        }
        if let Some((_, Some(depth))) = open {
            current_css.extend(std::iter::repeat_n('}', depth));
        }
        current_css
    }

    fn break_point(&self, level: u8) -> u16 {
        self.theme
            .breakpoints
            .get(level as usize)
            .copied()
            .unwrap_or_else(|| self.theme.breakpoints.last().copied().unwrap_or(0))
    }

    /// Open the blocks `wrapper` needs and return how many to close, or `None`
    /// when the breakpoint and the media query can never hold together.
    fn open_wrapper(&self, css: &mut String, wrapper: Wrapper<'_>) -> Option<usize> {
        let mut rules: Vec<(AtRuleKind, Cow<'_, str>)> =
            wrapper
                .at_rule
                .map_or_else(Vec::new, |(outer, kind, query)| {
                    outer
                        .iter()
                        .map(|rule| (rule.kind, Cow::Borrowed(rule.query.as_str())))
                        .chain(std::iter::once((kind, Cow::Borrowed(query))))
                        .collect()
                });
        if wrapper.level > 0 {
            let bp_query = format!("(min-width:{}px)", self.break_point(wrapper.level));
            // The breakpoint folds into the outermost `@media` when one query can
            // express both; otherwise it wraps the whole chain.
            let combined = match rules.first() {
                Some((AtRuleKind::Media, query)) => combine_media_queries(&bp_query, query),
                _ => MediaCombination::Nest,
            };
            match combined {
                MediaCombination::Merged(merged) => rules[0].1 = Cow::Owned(merged),
                MediaCombination::Never => return None,
                MediaCombination::Nest => {
                    rules.insert(0, (AtRuleKind::Media, Cow::Owned(bp_query)));
                }
            }
        }
        for (kind, query) in &rules {
            let _ = write_at_rule(css, *kind, query);
            css.push('{');
        }
        Some(rules.len())
    }

    fn write_global_props(&self, css: &mut String, mut global_props: Vec<GlobalProp<'_>>) {
        // Same order as class rules: selector group before breakpoint level, so a
        // `:hover` set at a wider breakpoint still precedes `:active`.
        global_props.sort_by(|a, b| {
            global_selector_group(a.1)
                .cmp(&global_selector_group(b.1))
                .then_with(|| a.0.cmp(&b.0))
                .then_with(|| a.1.cmp(b.1))
                .then_with(|| a.2.property.cmp(&b.2.property))
                .then_with(|| a.2.value.cmp(&b.2.value))
        });

        let mut open_level: Option<u8> = None;
        let mut open_selector: Option<&str> = None;
        for (level, selector, prop) in global_props {
            if open_level != Some(level) {
                if open_selector.take().is_some() {
                    css.push('}');
                }
                if open_level.is_some_and(|open| open > 0) {
                    css.push('}');
                }
                if level > 0 {
                    push_fmt!(css, "@media(min-width:{}px){{", self.break_point(level));
                }
                open_level = Some(level);
            }
            if open_selector == Some(selector) {
                css.push(';');
            } else {
                if open_selector.is_some() {
                    css.push('}');
                }
                css.push_str(selector);
                css.push('{');
                open_selector = Some(selector);
            }
            prop.write_declaration(css);
        }
        if open_selector.is_some() {
            css.push('}');
        }
        if open_level.is_some_and(|open| open > 0) {
            css.push('}');
        }
    }

    #[inline]
    fn create_header() -> &'static str {
        &HEADER
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

        if write_global {
            let mut style_orders: BTreeSet<u8> = BTreeSet::new();
            // Aggregate the `order == 0` base props by BORROWED reference rather than
            // deep-cloning each `StyleSheetProperty` (3 owned `String`s + selector) just to
            // hand `create_style_with_layers` refs it would only borrow. The set element is
            // `&StyleSheetProperty`, so cross-file dedup is preserved (values hash/eq the same
            // as the owned set), and the emitted CSS stays byte-identical.
            let mut base_styles = BTreeMap::<u8, FxHashSet<&StyleSheetProperty>>::new();
            self.properties.values().for_each(|map| {
                // Single walk of the top-level order map: record non-empty style
                // orders AND fold the `order == 0` base bucket in the same pass,
                // dropping the separate `map.get(&0)` probe per file. Output is
                // byte-identical (same `style_orders` set and `base_styles` map).
                for (order, props) in map {
                    if !props.is_empty() {
                        style_orders.insert(*order);
                    }
                    if *order == 0 {
                        props.iter().for_each(|prop| {
                            base_styles
                                .entry(*prop.0)
                                .or_default()
                                .extend(prop.1.iter());
                        });
                    }
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
            let mut layered_styles: LayeredStyles = BTreeMap::new();
            let base_css = if self.atom_plan.is_some() {
                let base_styles: BTreeMap<u8, FxHashSet<StyleSheetProperty>> = base_styles
                    .iter()
                    .map(|(level, props)| {
                        (
                            *level,
                            props.iter().map(|prop| prop.emission_identity()).collect(),
                        )
                    })
                    .collect();
                self.create_style_with_layers(&base_styles, Some(&mut layered_styles))
            } else {
                self.create_style_with_layers(&base_styles, Some(&mut layered_styles))
            };
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

                for (layer_name, layer_css) in layered_styles {
                    push_fmt!(&mut css, "@layer {layer_name}{{{layer_css}}}");
                }
            }
            // Atom hoisting: emit shared (hoisted) order!=0 atoms into the global
            // stylesheet, aggregated across every file and deduplicated by atom
            // identity (class_name).
            {
                let mut aggregated: BTreeMap<u8, BTreeMap<u8, FxHashSet<StyleSheetProperty>>> =
                    BTreeMap::new();
                for property_map in self.properties.values() {
                    for (style_order, level_map) in property_map {
                        if *style_order == 0 {
                            continue;
                        }
                        for (level, props) in level_map {
                            for prop in props {
                                if prop.hoisted {
                                    aggregated
                                        .entry(*style_order)
                                        .or_default()
                                        .entry(*level)
                                        .or_default()
                                        .insert(prop.emission_identity());
                                }
                            }
                        }
                    }
                }
                for (style_order, map) in aggregated {
                    let current_css = self.create_style(&map);
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

        // Bind the `Option<&str>` filename key once: both the keyframes and
        // properties lookups below use the same `""`-defaulted key, so computing
        // `unwrap_or_default()` a single time removes the duplicated call and
        // documents that both maps are keyed identically.
        let fkey = filename.unwrap_or_default();
        if let Some(keyframes) = self.keyframes.get(fkey) {
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
        if let Some(maps) = self.properties.get(fkey) {
            for (style_order, map) in maps {
                if *style_order == 0 {
                    // base style was created in global css
                    continue;
                }
                // Under atom hoisting, hoisted atoms were emitted globally; the
                // per-route chunk keeps only its route-private atoms.
                let current_css = {
                    // Common case: none of this map's atoms were hoisted, so the
                    // filtered map would equal the original — skip the clone-collect
                    // entirely and borrow `map` directly.
                    let any_hoisted = map.values().flatten().any(|prop| prop.hoisted);
                    if any_hoisted {
                        let filtered: BTreeMap<u8, FxHashSet<StyleSheetProperty>> = map
                            .iter()
                            .filter_map(|(level, props)| {
                                let kept: FxHashSet<StyleSheetProperty> =
                                    props.iter().filter(|prop| !prop.hoisted).cloned().collect();
                                (!kept.is_empty()).then_some((*level, kept))
                            })
                            .collect();
                        if filtered.is_empty() {
                            continue;
                        }
                        self.create_style(&filtered)
                    } else {
                        self.create_style(map)
                    }
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
    #[serial]
    #[case("1px", "1px")]
    #[case("$var", "var(--var)")]
    #[case("$var $var", "var(--var) var(--var)")]
    #[case("1px solid $red", "1px solid var(--red)")]
    // Test dot notation theme variables (e.g., $primary.100)
    // Dots should be converted to dashes for CSS variable names
    #[case("$primary.100", "var(--primary-100)")]
    #[case("$gray.200 $blue.500", "var(--gray-200) var(--blue-500)")]
    #[case("1px solid $border.primary", "1px solid var(--border-primary)")]
    #[case("$-", "$-")]
    #[case("$text-primary", "var(--text-primary)")]
    #[case("1px solid $line-color.100", "1px solid var(--line-color-100)")]
    // Test deep nested dot notation
    #[case("$color.brand.primary.100", "var(--color-brand-primary-100)")]
    fn test_convert_theme_variable_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(convert_theme_variable_value(input), expected);
    }

    #[test]
    #[serial]
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
    #[serial]
    fn create_css_atom_hoisting_emission() {
        use css::atom_hoist::set_atom_hoist;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};

        reset_class_map();
        reset_file_map();
        reset_file_routes();
        css::atom_hoist::restore_atom_plan(None);
        let mut routes = HashMap::new();
        routes.insert("a.tsx".to_string(), HashSet::from([0u32, 1]));
        routes.insert("b.tsx".to_string(), HashSet::from([0u32, 1]));
        routes.insert("private.tsx".to_string(), HashSet::from([0u32]));
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
        // Hoisted layered global atom (style_order 2). Hoist emission does not
        // collect custom layers, so create_style emits this directly in o2.
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
        let at = StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "(hover:hover)".to_string(),
            selector: None,
            outer: vec![],
            file: None,
        };
        sheet.add_property(
            "atr",
            "color",
            1,
            "blue",
            Some(&at),
            Some(255),
            Some("private.tsx"),
        );

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
        let chunk_css = sheet.create_css(Some("private.tsx"), false);
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
        css::atom_hoist::restore_atom_plan(None);
    }

    #[test]
    #[serial]
    fn create_css_hoists_layered_global_atom_as_direct_css() {
        use css::atom_hoist::set_atom_hoist;
        use css::file_routes::{get_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};

        let previous_threshold = css::atom_hoist::atom_hoist_threshold();
        let previous_plan = atom_plan();
        css::atom_hoist::restore_atom_plan(None);
        let previous_routes = get_file_routes();
        set_atom_hoist(Some(1));
        set_file_routes(HashMap::from([(
            "test.tsx".to_string(),
            HashSet::from([0u32]),
        )]));

        let mut sheet = StyleSheet::default();
        sheet.add_property_with_layer(
            "hg",
            "border-radius",
            0,
            "9px",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(2),
            Some("test.tsx"),
            Some("components"),
        );

        let css = sheet.create_css(None, false);
        set_atom_hoist(previous_threshold);
        set_file_routes(previous_routes);
        css::atom_hoist::restore_atom_plan(previous_plan);

        assert!(
            css.contains("@layer o2{@layer components{div{border-radius:9px}}}"),
            "layered global hoist must keep its layer: {css}"
        );
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    fn wrong_breakpoint() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin-left", 10, "40px", None, None, None);
        sheet.add_property("test", "margin-right", 10, "40px", None, None, None);
        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    fn test_motion_reduce_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "display",
            0,
            "none",
            Some(&"motion-reduce".into()),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
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
    #[serial]
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
                outer: vec![],
                file: None,
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
                outer: vec![],
                file: None,
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
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
                outer: vec![],
                file: None,
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
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
                outer: vec![],
                file: None,
            }),
            None,
            None,
        );

        assert_debug_snapshot!(sheet.create_css(None, false).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
    fn test_deserialize() {
        {
            let sheet: StyleSheet = serde_json::from_str(
                r##"{
            "atomNamingVersion": 4,
            "names": {}, "atom_plan": null, "keyframes": {},
            "global_css_files": [], "imports": {}, "font_faces": {},
            "sourceIds": {}, "classMap": {}, "fileMap": {},
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
            .unwrap_or_else(|error| panic!("{error}"));
            assert_debug_snapshot!(sheet);
        }

        {
            let sheet: Result<StyleSheet, _> = serde_json::from_str(
                r##"{
            "atomNamingVersion": 4,
            "names": {}, "atom_plan": null, "keyframes": {},
            "global_css_files": [], "imports": {}, "font_faces": {},
            "sourceIds": {}, "classMap": {}, "fileMap": {},
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
        sheet
            .update_styles(&FxHashSet::default(), "index.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_debug_snapshot!(
            sheet
                .create_css(Some("index.tsx"), true)
                .split("*/")
                .nth(1)
                .unwrap()
        );

        let mut sheet = StyleSheet::default();
        let output = extract(
            "index.tsx",
            "import {Box,globalCss,keyframes,Flex} from '@devup-ui/core';<Flex/>;keyframes({from:{opacity:0},to:{opacity:1}});<Box w={1} h={variable} />;globalCss`div{color:red}`;globalCss({div:{display:'flex'},imports:['https://test.com/a.css'],fontFaces:[{fontFamily:'Roboto',src:'url(/fonts/Roboto-Regular.ttf)'}]})",
            ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: std::collections::HashMap::new() },
        )
        .unwrap();
        sheet
            .update_styles(&output.styles, "index.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_debug_snapshot!(sheet.create_css(None, true).split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
    fn test_class_names_do_not_depend_on_the_order_styles_are_inserted() {
        use extractor::extract_style::extract_static_style::ExtractStaticStyle;
        use extractor::extract_style::extract_style_value::ExtractStyleValue;

        let values: Vec<ExtractStyleValue> = (0..40)
            .map(|index| {
                ExtractStyleValue::Static(ExtractStaticStyle::new(
                    ["color", "margin", "padding", "width"][index % 4],
                    &format!("{index}px"),
                    0,
                    None,
                ))
            })
            .collect();
        let mut outputs = Vec::new();
        for reverse in [false, true] {
            css::class_map::reset_class_map();
            let mut styles = FxHashSet::default();
            if reverse {
                values.iter().rev().for_each(|value| {
                    styles.insert(value.clone());
                });
            } else {
                values.iter().for_each(|value| {
                    styles.insert(value.clone());
                });
            }
            let mut sheet = StyleSheet::default();
            sheet
                .update_styles(&styles, "index.tsx", true)
                .unwrap_or_else(|error| panic!("{error}"));
            outputs.push((
                sheet.create_css(None, true),
                css::class_map::get_class_map(),
            ));
        }
        assert_eq!(outputs[0], outputs[1]);
    }
    #[test]
    #[serial]
    fn test_update_styles_with_typography() {
        use extractor::extract_style::extract_style_value::ExtractStyleValue;

        let mut sheet = StyleSheet::default();
        let mut styles = FxHashSet::default();
        styles.insert(ExtractStyleValue::Typography("$heading".to_string()));
        let (collected, updated) = sheet
            .update_styles(&styles, "index.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        // Typography doesn't collect or update
        assert!(!collected);
        assert!(!updated);
    }

    #[test]
    #[serial]
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
    #[serial]
    fn test_custom_layer_keeps_selector_order() {
        let mut sheet = StyleSheet::default();
        for (selector, value) in [("a:active", "blue"), ("a:hover", "red"), ("a", "black")] {
            sheet.add_property_with_layer(
                "a",
                "color",
                0,
                value,
                Some(&StyleSelector::Global(
                    selector.to_string(),
                    "links.css.ts".to_string(),
                )),
                Some(0),
                None,
                Some("ui"),
            );
        }
        let css = sheet.create_css(None, false);
        assert!(
            css.contains("@layer ui{a{color:black}a:hover{color:red}a:active{color:blue}}"),
            "{css}"
        );
    }

    #[test]
    #[serial]
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
                outer: vec![],
                file: None,
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
    #[serial]
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
                outer: vec![],
                file: None,
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
    #[serial]
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
    #[serial]
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
                outer: vec![],
                file: None,
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
    #[serial]
    fn test_stylesheet_css_struct() {
        let css_entry = StyleSheetCss {
            css: "div{display:flex}".to_string(),
        };
        assert_eq!(css_entry.css, "div{display:flex}");

        let empty = StyleSheetCss { css: String::new() };
        assert_eq!(empty.css, "");
    }

    #[test]
    #[serial]
    fn test_stylesheet_property_ord_no_selectors() {
        // Both sides without selectors: branches on property then value.
        let make = |property: &str, value: &str| StyleSheetProperty {
            class_name: "a".to_string(),
            property: property.to_string(),
            value: value.to_string(),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: false,
        };
        assert_eq!(make("color", "red").cmp(&make("color", "red")), Equal);
        assert!(make("color", "red") < make("color", "white"));
        assert!(make("color", "red") < make("display", "block"));
        assert!(make("display", "block") > make("color", "white"));
    }

    #[test]
    #[serial]
    fn test_stylesheet_property_ord_with_selectors() {
        let make =
            |selector: Option<StyleSelector>, property: &str, value: &str| StyleSheetProperty {
                class_name: "a".to_string(),
                property: property.to_string(),
                value: value.to_string(),
                selector,
                layer: None,
                typography: false,
                hoisted: false,
                owner_reset: false,
            };
        let hover = || Some(StyleSelector::Selector("&:hover".to_string()));

        assert!(make(hover(), "color", "red") < make(hover(), "color", "white"));
        assert!(make(hover(), "color", "red") < make(hover(), "display", "block"));
        assert_ne!(
            make(hover(), "color", "red").cmp(&make(
                Some(StyleSelector::Selector("&:focus".to_string())),
                "color",
                "red"
            )),
            Equal
        );
        assert!(make(None, "color", "red") < make(hover(), "color", "red"));
    }

    #[test]
    #[serial]
    fn test_global_selector_group() {
        assert_eq!(global_selector_group("body"), (false, 0));
        assert_eq!(global_selector_group("a:hover"), (true, 0));
        assert_eq!(global_selector_group("a:active"), (true, 3));
        assert_eq!(global_selector_group(":root"), (true, 0));
    }

    #[test]
    #[serial]
    fn test_existing_collection_buckets_are_reused() {
        let mut sheet = StyleSheet::default();
        assert!(sheet.add_property("a", "color", 0, "red", None, None, Some("test.tsx")));
        assert!(sheet.add_property("b", "display", 0, "block", None, None, Some("test.tsx")));

        sheet.add_import("test.tsx", "base.css");
        sheet.add_import("test.tsx", "theme.css");

        let first_font = BTreeMap::from([("font-family".to_string(), "First".to_string())]);
        let second_font = BTreeMap::from([("font-family".to_string(), "Second".to_string())]);
        sheet.add_font_face("test.tsx", &first_font);
        sheet.add_font_face("test.tsx", &second_font);

        assert!(sheet.add_css("test.tsx", "html { color:red }"));
        assert!(sheet.add_css("test.tsx", "body { color:blue }"));

        assert_eq!(sheet.properties["test.tsx"].len(), 1);
        assert_eq!(sheet.imports["test.tsx"].len(), 2);
        assert_eq!(sheet.font_faces["test.tsx"].len(), 2);
        assert_eq!(sheet.css["test.tsx"].len(), 2);
    }

    #[test]
    #[serial]
    fn base_styles_emit_globally_without_atom_promotion() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("base", "color", 0, "red", None, Some(0), Some("test.tsx"));

        assert!(
            sheet
                .create_css(None, false)
                .contains(concat!(".base{", "color:red}"))
        );
        assert!(
            !sheet
                .create_css(Some("test.tsx"), false)
                .contains("color:red")
        );
    }

    #[test]
    #[serial]
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

        let (collected, _) = sheet
            .update_styles(&styles, "test.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
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

        let (collected, _) = sheet
            .update_styles(&styles, "test.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
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

        let (collected, _) = sheet
            .update_styles(&styles, "test.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(collected);

        let css = sheet.create_css(None, false);
        assert!(css.contains("box-shadow:0 1px 2px #0003"));
    }

    #[test]
    #[serial]
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

    fn pipeline_css(theme: Theme, source: &str) -> (String, String) {
        css::debug::set_debug(false);
        css::set_prefix(None);
        css::atom_hoist::set_atom_hoist(None);
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        let output = extract(
            "test.tsx",
            &format!("import {{Box,css,styled,globalCss}} from '@devup-ui/core';\n{source}"),
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: std::collections::HashMap::new(),
            },
        )
        .unwrap();
        sheet
            .update_styles(&output.styles, "test.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        // Counter/content identities are immaterial here; first CSS appearance
        // assigns an injective label shared by the CSS and generated code.
        let mut names: Vec<String> = vec![];
        let css = compile_regex(r"(@layer [^{;]+)|\.([A-Za-z_][\w-]*)")
            .replace_all(
                sheet.create_css(None, false).split("*/").nth(1).unwrap(),
                |caps: &regex_lite::Captures| {
                    if let Some(layer) = caps.get(1) {
                        return layer.as_str().to_string();
                    }
                    let index = names.iter().position(|n| *n == caps[2]).unwrap_or_else(|| {
                        names.push(caps[2].to_string());
                        names.len() - 1
                    });
                    format!(".c{index}")
                },
            )
            .into_owned();
        let code = compile_regex(r"[A-Za-z_][\w-]*")
            .replace_all(&output.code, |caps: &regex_lite::Captures| {
                names
                    .iter()
                    .position(|name| *name == caps[0])
                    .map_or_else(|| caps[0].to_string(), |index| format!("c{index}"))
            })
            .into_owned();
        // Only generated site variables are relabeled, never theme/manual vars.
        // Exact token matching preserves distinct sites and every use/reset.
        let mut variables: Vec<String> = vec![];
        let variable_regex = compile_regex(r"---S[\w-]+");
        let css = variable_regex
            .replace_all(&css, |caps: &regex_lite::Captures| {
                let index = variables
                    .iter()
                    .position(|name| *name == caps[0])
                    .unwrap_or_else(|| {
                        variables.push(caps[0].to_string());
                        variables.len() - 1
                    });
                format!("---v{index}")
            })
            .into_owned();
        let code = variable_regex
            .replace_all(&code, |caps: &regex_lite::Captures| {
                let index = variables.iter().position(|name| *name == caps[0]).unwrap();
                format!("---v{index}")
            })
            .into_owned();
        (crate::sheet_test_code::normalize_bindings(&code), css)
    }

    #[test]
    #[serial]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn test_at_rule_pipeline() {
        for (source, expected) in [
            // Conditions (at-rules) win over breakpoint values; pseudo states keep
            // `SELECTOR_ORDER` across breakpoints.
            (
                "<Box bg={['red', null, 'blue']} _hover={{ bg: 'green' }} />",
                ".c0{background:red}@media(min-width:768px){.c1{background:blue}}.c2:hover{background:green}",
            ),
            (
                "<Box _hover={{ bg: ['red', null, 'blue'] }} _active={{ bg: 'green' }} />",
                ".c0:hover{background:red}@media(min-width:768px){.c1:hover{background:blue}}.c2:active{background:green}",
            ),
            (
                "<Box transition={['opacity .2s', null, 'all .3s']} _motionReduce={{ transition: 'none' }} />",
                ".c0{transition:opacity .2s}@media(min-width:768px){.c1{transition:all .3s}}@media(prefers-reduced-motion:reduce){.c2{transition:none}}",
            ),
            (
                "<Box bg={['red', null, 'blue']} _supports={{ '(display: grid)': { bg: 'green' } }} />",
                ".c0{background:red}@media(min-width:768px){.c1{background:blue}}@supports(display:grid){.c2{background:green}}",
            ),
            (
                "<Box _media={{ '(min-width: 1000px)': { p: 2 }, '(min-width: 500px)': { p: 1 }, '(max-width: 300px)': { p: 3 }, '(max-width: 900px)': { p: 4 } }} />",
                "@media(min-width:500px){.c0{padding:4px}}@media(min-width:1000px){.c1{padding:8px}}@media(max-width:900px){.c2{padding:16px}}@media(max-width:300px){.c3{padding:12px}}",
            ),
            (
                "<Box _active={{ _motionReduce: { bg: 'a' } }} _hover={{ _motionReduce: { bg: 'b' } }} />",
                "@media(prefers-reduced-motion:reduce){.c0:hover{background:b}.c1:active{background:a}}",
            ),
            // Nesting in either direction keeps both the condition and the selector.
            (
                "<Box _hover={{ _print: { bg: 'red' } }} _print={{ _focus: { color: 'red' } }} />",
                "@media print{.c0:hover{background:red}.c1:focus{color:red}}",
            ),
            (
                "<Box _motionSafe={{ _hover: { transform: 'scale(1.05)' }, _themeDark: { color: 'red' }, _groupHover: { color: 'blue' } }} />",
                "@media(prefers-reduced-motion:no-preference){.c0:hover{transform:scale(1.05)}:is([role=group],[data-group]):hover .c1{color:blue}:root[data-theme=dark] .c2{color:red}}",
            ),
            (
                "<Box _media={{ '(prefers-reduced-motion: reduce)': { selectors: { '&:hover': { bg: 'red' } }, _before: { content: '\"\"' } } }} />",
                "@media(prefers-reduced-motion:reduce){.c0:hover{background:red}.c1::before{content:\"\"}}",
            ),
            (
                "<Box _hover={{ _themeDark: { bg: 'red' } }} />",
                ":root[data-theme=dark] .c0:hover{background:red}",
            ),
            // Nested at-rules fold into one query or nest; exclusive ones are dropped.
            (
                "<Box _print={{ _motionReduce: { bg: 'red' }, _screen: { bg: 'blue' } }} />",
                "@media print and (prefers-reduced-motion:reduce){.c0{background:red}}",
            ),
            (
                "<Box _media={{ '(prefers-reduced-motion: reduce)': { _supports: { '(display: grid)': { bg: 'red' } } } }} />",
                "@media(prefers-reduced-motion:reduce){@supports(display:grid){.c0{background:red}}}",
            ),
            // Breakpoints combine with the query: media type first, lists distribute,
            // `not print` becomes `screen`, anything else nests.
            (
                "<Box _print={{ bg: ['red', null, 'blue'] }} />",
                "@media print{.c0{background:red}}@media print and (min-width:768px){.c1{background:blue}}",
            ),
            (
                "<Box _media={{ '(orientation: portrait), (hover: none)': { bg: [null, null, 'blue'] }, 'not print': { color: [null, null, 'red'] } }} />",
                "@media screen and (min-width:768px){.c0{color:red}}@media(min-width:768px)and (orientation:portrait),(min-width:768px)and (hover:none){.c1{background:blue}}",
            ),
            (
                "<Box _media={{ 'not print and (color)': { bg: [null, null, 'blue'] }, 'not all': { color: [null, null, 'red'] } }} />",
                "@media(min-width:768px){@media not print and (color){.c0{background:blue}}}",
            ),
            (
                "<Box _supports={{ 'not (display: grid)': { bg: [null, null, 'blue'] } }} />",
                "@media(min-width:768px){@supports not (display:grid){.c0{background:blue}}}",
            ),
            // Emotion-style object keys and template literals.
            (
                "<div className={css({ '@media print': { color: 'blue', '&:hover': { color: 'green' } }, ':focus': { color: 'red' } })} />",
                ".c0:focus{color:red}@media print{.c1{color:blue}.c2:hover{color:green}}",
            ),
            (
                "<div className={css`transition: all .3s; @media (prefers-reduced-motion: reduce) { transition: none; } &:hover { @media print { color: red; } span { color: blue; } }`} />",
                ".c0{transition:all .3s}.c1:hover span{color:blue}@media print{.c2:hover{color:red}}@media(prefers-reduced-motion:reduce){.c3{transition:none}}",
            ),
            (
                "<Box className=\"print:motion-reduce:hidden print:screen:flex\" />",
                "@media print and (prefers-reduced-motion:reduce){.c0{display:none}}",
            ),
            (
                "globalCss({ '@media (prefers-reduced-motion: reduce)': { '*': { transition: 'none' } }, _print: { body: { bg: 'white', _hover: { color: 'red' } } }, _media: { '(min-width: 768px)': { body: { m: 2 } } }, _screen: { _print: { body: { color: 'red' } } } })",
                "@layer b;@layer b{@media(min-width:768px){body{margin:8px}}@media print{body{background:white}body:hover{color:red}}@media(prefers-reduced-motion:reduce){*{transition:none}}}",
            ),
            (
                "<Box selectors={{ _print: { bg: 'red' }, '@media (hover: none)': { color: 'red' }, '&:hover, &:focus': { m: 1 } }} />",
                ".c0:hover{margin:4px}.c1:focus{margin:4px}@media print{.c2{background:red}}@media(hover:none){.c3{color:red}}",
            ),
            // Global pseudo rules keep `SELECTOR_ORDER` across breakpoints.
            (
                "globalCss({ a: { _hover: { color: 'red' }, _active: { color: 'blue' }, _focus: { color: 'green' } } })",
                "@layer b;@layer b{a:hover{color:red}a:focus{color:green}a:active{color:blue}}",
            ),
            (
                "globalCss({ a: { color: 'black', _hover: { color: [null, null, 'red'] }, _active: { color: 'blue' } } })",
                "@layer b;@layer b{a{color:black}@media(min-width:768px){a:hover{color:red}}a:active{color:blue}}",
            ),
            (
                "globalCss({ a: { color: ['red', null, 'blue'] } })",
                "@layer b;@layer b{a{color:red}@media(min-width:768px){a{color:blue}}}",
            ),
        ] {
            assert_eq!(
                pipeline_css(Theme::default(), source).1,
                expected,
                "{source}"
            );
        }
    }

    #[test]
    #[serial]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn test_layer_pipeline() {
        for (source, expected) in [
            // A layer keeps breakpoints, selectors and at-rules of its declarations.
            (
                "globalCss({ '*': { '@layer': 'reset', margin: [0, null, 4], _hover: { color: ['red', null, 'blue'] }, _print: { color: 'black' } }, body: { color: 'white' } })",
                "@layer b;@layer b{body{color:white}}@layer reset;@layer reset{*{margin:0}@media(min-width:768px){*{margin:16px}}*:hover{color:red}@media(min-width:768px){*:hover{color:blue}}@media print{*{color:black}}}",
            ),
            (
                "<div className={css({ color: 'red', '@layer': { base: { color: 'blue', p: [1, null, 2], _hover: { color: 'green' }, '@layer': { inner: { m: 1 } } } } })} />",
                ".c0{color:red}@layer base{.c1{color:blue}.c2{padding:4px}@media(min-width:768px){.c3{padding:8px}}.c4:hover{color:green}}@layer base.inner{.c5{margin:4px}}",
            ),
            // Layered and unlayered declarations keep apart, dynamic ones included.
            (
                "const A = styled.div({ color: 'red', width: w, '@layer': { base: { color: 'red', width: v } } })",
                ".c0{---v0:initial}.c1{color:red}.c2{---v1:initial;width:var(---v1)}@layer base{.c3{color:red}.c0{width:var(---v0)}}",
            ),
        ] {
            let (code, css) = pipeline_css(Theme::default(), source);
            assert_eq!(css, expected, "{source}");
            if source.starts_with("const A") {
                assert_eq!(
                    code,
                    concat!(
                        "import \"@devup-ui/core/devup-ui.css\";\n",
                        "const A = ((__capture0, __capture1) => ({ style, className, ...rest }) => <div {...rest} className={[\"c1 c2 c3 c0\", className].filter(Boolean).join(\" \")} style={{\n",
                        "\t...{\n\t\t\"---v1\": __capture0,\n\t\t\"---v0\": __capture1\n\t},\n\t...style\n}} />)(w, v);\n",
                    )
                );
            }
        }
    }

    #[test]
    #[serial]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn test_conditional_typography_pipeline() {
        for (source, expected) in [
            (
                "<Box _hover={{ typography: 'small' }} />",
                "@layer t;@layer t{.c2:hover{font-size:12px;line-height:1.2}}",
            ),
            (
                "<Box _motionReduce={{ typography: 'title' }} />",
                "@layer t;@layer t{@media(prefers-reduced-motion:reduce){.c2{font-family:var(--heading);font-size:20px;font-weight:700}}@media(min-width:768px)and (prefers-reduced-motion:reduce){.c2{font-size:32px}}}",
            ),
            (
                "<Box _hover={[null, null, { typography: 'title' }]} />",
                "@layer t;@layer t{@media(min-width:768px){.c2:hover{font-family:var(--heading);font-size:32px;font-weight:700}}}",
            ),
            (
                "<Box _hover={{ typography: size }} />",
                "@layer t;@layer t{.c2:hover{font-family:var(--heading);font-size:20px;font-weight:700}.c3:hover{font-size:12px;line-height:1.2}@media(min-width:768px){.c2:hover{font-size:32px}}}",
            ),
            (
                "<Box _hover={{ typography: `${size}` }} />",
                "@layer t;@layer t{.c2:hover{font-family:var(--heading);font-size:20px;font-weight:700}.c3:hover{font-size:12px;line-height:1.2}@media(min-width:768px){.c2:hover{font-size:32px}}}",
            ),
            ("<Box _hover={{ typography: 'missing' }} />", "@layer t;"),
            // A declaration written next to the preset wins over the preset's, at
            // every breakpoint from the one it starts at.
            (
                "<Box _hover={{ typography: 'small', fontSize: '11px' }} />",
                "@layer t;.c2:hover{font-size:11px}@layer t{.c3:hover{line-height:1.2}}",
            ),
            (
                "<Box _hover={{ typography: 'title', fontSize: '11px' }} />",
                "@layer t;.c2:hover{font-size:11px}@layer t{.c3:hover{font-family:var(--heading);font-weight:700}}",
            ),
            (
                "<Box _hover={{ typography: 'title', fontSize: [null, null, null, '11px'] }} />",
                "@layer t;@media(min-width:992px){.c2:hover{font-size:11px}}@layer t{.c3:hover{font-family:var(--heading);font-size:20px;font-weight:700}@media(min-width:768px){.c3:hover{font-size:32px}}}",
            ),
            (
                "<Box typography={['small', null, 'title']} fontSize=\"11px\" />",
                "@layer t;.c2{font-size:11px}@layer t{@media(min-width:768px){.c3{font-family:var(--heading);font-weight:700}}}",
            ),
            (
                "<Box _hover={{ typography: 'title', fontSize: size }} />",
                "@layer t;.c2{---v0:initial}.c2:hover{font-size:var(---v0)}@layer t{.c3:hover{font-family:var(--heading);font-weight:700}}",
            ),
            (
                "<Box _hover={{ typography: 'title', fontSize: cond ? '11px' : [null, null, '12px'] }} />",
                "@layer t;.c2:hover{font-size:11px}@media(min-width:768px){.c3:hover{font-size:12px}}@layer t{.c4:hover{font-family:var(--heading);font-size:20px;font-weight:700}}",
            ),
            (
                "<Box _hover={{ typography: 'title', fontSize: cond ? '11px' : undefined, lineHeight: { a: '1', b: '2' }[key] }} />",
                "@layer t;.c2:hover{font-size:11px}.c3:hover{line-height:1}.c4:hover{line-height:2}@layer t{.c5:hover{font-family:var(--heading);font-size:20px;font-weight:700}@media(min-width:768px){.c5:hover{font-size:32px}}}",
            ),
            (
                "<Box _hover={{ typography: cond ? 'small' : { a: 'title' }[key], fontSize: '11px' }} typography={size} positioning={pos} />",
                "@layer t;.c2{bottom:0}.c3{left:0}.c4{right:0}.c5{top:0}.c6:hover{font-size:11px}@layer t{.c7:hover{font-family:var(--heading);font-weight:700}.c8:hover{line-height:1.2}}",
            ),
            (
                "<Box _hover={{ typography: size, fontSize: '11px' }} />",
                "@layer t;.c2:hover{font-size:11px}@layer t{.c3:hover{font-family:var(--heading);font-weight:700}.c4:hover{line-height:1.2}}",
            ),
            (
                "globalCss({ body: { typography: 'small' } })",
                "@layer b,t;@layer b{@layer t{body{font-size:12px;line-height:1.2}}}",
            ),
            (
                "globalCss({ h1: { typography: 'title', fontSize: '13px' } })",
                "@layer b,t;@layer b{h1{font-size:13px}@layer t{h1{font-family:var(--heading);font-weight:700}}}",
            ),
        ] {
            let mut theme = Theme::default();
            theme.add_typography(
                "small",
                vec![Some(Typography::new(
                    None,
                    Some("12px".to_string()),
                    None,
                    Some("1.2".to_string()),
                    None,
                ))],
            );
            theme.add_typography(
                "title",
                vec![
                    Some(Typography::new(
                        Some("$heading".to_string()),
                        Some("20px".to_string()),
                        Some("700".to_string()),
                        None,
                        Some(" ".to_string()),
                    )),
                    None,
                    Some(Typography::new(
                        None,
                        Some("32px".to_string()),
                        None,
                        None,
                        None,
                    )),
                ],
            );
            let (code, css) = pipeline_css(theme, &format!("let size = 'small';\n{source}"));
            let expected_code = match source {
                "<Box _hover={{ typography: size }} />" => Some(
                    "<div className={{\n\t\"small\": \"c3\",\n\t\"title\": \"c2\"\n}[size] || \"\"} />;\n",
                ),
                "<Box _hover={{ typography: `${size}` }} />" => Some(
                    "<div className={{\n\t\"small\": \"c3\",\n\t\"title\": \"c2\"\n}[`${size}`] || \"\"} />;\n",
                ),
                "<Box _hover={{ typography: 'title', fontSize: size }} />" => {
                    Some("<div className=\"c3 c2\" style={{ \"---v0\": size }} />;\n")
                }
                "<Box _hover={{ typography: 'title', fontSize: cond ? '11px' : [null, null, '12px'] }} />" => {
                    Some(concat!(
                        "((__capture0) => <div className={`c4 ${__capture0?.[0] ?? \"\"}`} style={{ ...__capture0?.[1] }} />)(cond ? ((__devupValue) => [\n",
                        "\t\"c2\",\n\t{},\n\t__devupValue\n])(\"11px\") : ((__devupLevel0, __devupLevel1, __devupLevel2) => [\n",
                        "\t`${__devupLevel0?.[0] ?? \"\"} ${__devupLevel1?.[0] ?? \"\"} ${__devupLevel2?.[0] ?? \"\"}`,\n",
                        "\t{\n\t\t...__devupLevel0?.[1],\n\t\t...__devupLevel1?.[1],\n\t\t...__devupLevel2?.[1]\n\t},\n",
                        "\t[\n\t\t__devupLevel0?.[2],\n\t\t__devupLevel1?.[2],\n\t\t__devupLevel2?.[2]\n\t]\n",
                        "])(((__devupValue) => [\n\t\"\",\n\t{},\n\t__devupValue\n])(null), ((__devupValue) => [\n",
                        "\t\"\",\n\t{},\n\t__devupValue\n])(null), ((__devupValue) => [\n\t\"c3\",\n\t{},\n\t__devupValue\n])(\"12px\")));\n",
                    ))
                }
                "<Box _hover={{ typography: size, fontSize: '11px' }} />" => Some(
                    "<div className={`c2 ${{\n\t\"small\": \"c4\",\n\t\"title\": \"c3\"\n}[size] || \"\"}`} />;\n",
                ),
                "<Box _hover={{ typography: cond ? 'small' : { a: 'title' }[key], fontSize: '11px' }} typography={size} positioning={pos} />" => {
                    Some(concat!(
                        "((__capture0, __devupSpread0) => <div className={`${`c6 ${__capture0?.[0] ?? \"\"}`} ${__devupSpread0} ${{\n",
                        "\t\"bottom\": \"c2\",\n\t\"bottom-left\": \"c2 c3\",\n\t\"bottom-right\": \"c2 c4\",\n",
                        "\t\"left\": \"c3\",\n\t\"right\": \"c4\",\n\t\"top\": \"c5\",\n\t\"top-left\": \"c3 c5\",\n\t\"top-right\": \"c4 c5\"\n",
                        "}[pos] || \"\"}`} style={{ ...__capture0?.[1] }} />)(((__devupField0, __devupField1) => [\n",
                        "\t`${__devupField0?.[0] ?? \"\"} ${__devupField1?.[0] ?? \"\"}`,\n",
                        "\t{\n\t\t...__devupField0?.[1],\n\t\t...__devupField1?.[1]\n\t},\n",
                        "\t{\n\t\ttypography: __devupField0?.[2],\n\t\tfontSize: __devupField1?.[2]\n\t}\n",
                        "])(cond ? ((__devupValue) => [\n\t\"c8\",\n\t{},\n\t__devupValue\n])(\"small\") : { a: ((__devupValue) => [\n",
                        "\t\"c7\",\n\t{},\n\t__devupValue\n])(\"title\") }[key], ((__devupValue) => [\n\t\"\",\n\t{},\n\t__devupValue\n])(\"11px\")), ",
                        "((__devupTypography) => __devupTypography ? `typo-${__devupTypography}` : \"\")(size));\n",
                    ))
                }
                _ => None,
            };
            if let Some(expected_code) = expected_code {
                assert_eq!(
                    code,
                    format!(
                        "import \"@devup-ui/core/devup-ui.css\";\nlet size = \"small\";\n{expected_code}"
                    ),
                    "{source}"
                );
            }
            // Drop the theme's own `.typo-*` layer; only the conditional atoms matter here.
            let start = css.find("@layer t{").unwrap();
            let mut depth = 0;
            let end = css[start..]
                .char_indices()
                .find_map(|(index, c)| {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    (c == '}' && depth == 0).then_some(start + index + 1)
                })
                .unwrap();
            assert_eq!(
                format!("{}{}", &css[..start], &css[end..]),
                expected,
                "{source}\n{code}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_global_css_reads_theme_tokens() {
        let mut sheet = StyleSheet::default();
        let output = extract(
            "global.tsx",
            "import {globalCss} from '@devup-ui/core';globalCss({ body: { color: '$text', border: '1px solid $line.100' } })",
            ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: std::collections::HashMap::new() },
        )
        .unwrap();
        sheet
            .update_styles(&output.styles, "global.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        let css = sheet.create_css(None, false);
        assert!(
            css.contains("body{border:1px solid var(--line-100);color:var(--text)}"),
            "{css}"
        );
    }

    #[test]
    #[serial]
    fn test_rm_global_css_drops_global_at_rules() {
        let mut sheet = StyleSheet::default();
        let output = extract(
            "global.tsx",
            "import {globalCss} from '@devup-ui/core';globalCss({ body: { color: 'red', _motionReduce: { transition: 'none' } } })",
            ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: std::collections::HashMap::new() },
        )
        .unwrap();
        sheet
            .update_styles(&output.styles, "global.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(sheet.create_css(None, false).contains("transition:none"));

        assert!(sheet.rm_global_css("global.tsx", true));
        let css = sheet.create_css(None, false);
        assert!(!css.contains("transition:none"), "{css}");
        assert!(!css.contains("color:red"), "{css}");
    }

    #[test]
    #[serial]
    fn test_tailwind_classes_css() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        for (file, code) in [
            (
                "a.tsx",
                "import {Box} from '@devup-ui/core'\n<Box className=\"translate-x-4 hover:focus:mt-4 card\" />",
            ),
            (
                "b.tsx",
                "import {Box} from '@devup-ui/core'\n<Box className=\"translate-y-2 before:inline-block text-sm\" />",
            ),
        ] {
            let output = extract(
                file,
                code,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: std::collections::HashMap::new(),
                },
            )
            .unwrap();
            sheet.update_styles(&output.styles, file, true).unwrap();
        }
        let css = sheet.create_css(None, false);
        // The registrations are written once however many files use them
        assert_eq!(
            css.matches("@property --tw-translate-x{").count(),
            1,
            "{css}"
        );
        assert!(
            css.contains("@property --tw-content{syntax:\"*\";inherits:false;initial-value:\"\"}"),
            "{css}"
        );
        for rule in [
            ".OHa6rxgmn20xxa3xjy{--tw-translate-x:1rem}",
            ".OHbthgbe54ph87kjzs{--tw-translate-y:.5rem}",
            ".OHca1k0lv7u879ersf{translate:var(--tw-translate-x) var(--tw-translate-y)}",
            ".OHcajqalo_p20xmjh8{font-size:.875rem}",
            ".OHbkdbnvb9xhfqwzek{line-height:var(--tw-leading,calc(1.25 / .875))}",
            ".OHb8o11h_bnuqguo2o::before{content:var(--tw-content)}",
            ".OHbohq0bo7m9z_n4o6::before{display:inline-block}",
            "@media(hover:hover){.OHa5ynunt1t3zk0t5h:hover:focus{margin-top:1rem}}",
        ] {
            assert!(css.contains(rule), "{rule} in {css}");
        }
    }

    #[test]
    #[serial]
    fn test_dynamic_base_style_updates_base_sheet() {
        reset_class_map();
        reset_file_map();
        let mut sheet = StyleSheet::default();
        let output = extract(
            "test.tsx",
            "import {Box} from '@devup-ui/core'\n<Box styleOrder={0} bg={color} />",
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: std::collections::HashMap::new(),
            },
        )
        .unwrap();
        assert_eq!(
            sheet
                .update_styles(&output.styles, "test.tsx", true)
                .unwrap_or_else(|error| panic!("{error}")),
            (true, true)
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
let color = "red";
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

        let (collected, _) = sheet
            .update_styles(&output.styles, "test.tsx", true)
            .unwrap_or_else(|error| panic!("{error}"));
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
