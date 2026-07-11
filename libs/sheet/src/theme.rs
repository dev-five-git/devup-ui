use css::optimize_value::optimize_value;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

/// `ColorEntry` stores both the original key (for TypeScript interface) and CSS key (for CSS variables)
#[derive(Debug, Clone, Serialize)]
pub struct ColorEntry {
    /// Original key with dots for TypeScript interface (e.g., "gray.100")
    pub interface_key: String,
    /// CSS variable key with dashes (e.g., "gray-100")
    pub css_key: String,
    /// Color value
    pub value: String,
}

/// `ColorTheme` stores flattened color entries
/// Supports:
/// - Simple: `primary: "#000"` -> `interface_key`: "primary", `css_key`: "primary"
/// - Dot notation: `"primary.100": "#000"` -> `interface_key`: "primary.100", `css_key`: "primary-100"
/// - Nested object: `hello: { 100: "#000" }` -> `interface_key`: "hello.100", `css_key`: "hello-100"
/// - Deep nested: `gray: { light: { 100: "#000" } }` -> `interface_key`: "gray.light.100", `css_key`: "gray-light-100"
#[derive(Default, Serialize, Debug)]
pub struct ColorTheme {
    /// Map from `css_key` to `ColorEntry` for quick lookup
    entries: HashMap<String, ColorEntry>,
}

/// Derive the CSS-variable key from a raw name: dots become dashes.
/// `replace('.', "-")` always allocates + scans even for the common dot-free
/// name, so borrow the name as-is when it has no `.`. Byte-identical output.
fn css_key_from(name: &str) -> Cow<'_, str> {
    if name.contains('.') {
        Cow::Owned(name.replace('.', "-"))
    } else {
        Cow::Borrowed(name)
    }
}

/// Recursively flatten a JSON value into `ColorEntry` list
/// `interface_prefix` uses dots, `css_prefix` uses dashes
fn flatten_color_value(
    interface_prefix: &str,
    css_prefix: &str,
    value: &Value,
    result: &mut HashMap<String, ColorEntry>,
) -> Result<(), String> {
    match value {
        Value::String(s) => {
            result.insert(
                css_prefix.to_string(),
                ColorEntry {
                    interface_key: interface_prefix.to_string(),
                    css_key: css_prefix.to_string(),
                    value: s.clone(),
                },
            );
            Ok(())
        }
        Value::Object(obj) => {
            for (key, val) in obj {
                let new_interface_prefix = if interface_prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{interface_prefix}.{key}")
                };
                let key_css = css_key_from(key);
                let new_css_prefix = if css_prefix.is_empty() {
                    key_css.into_owned()
                } else {
                    format!("{css_prefix}-{key_css}")
                };
                flatten_color_value(&new_interface_prefix, &new_css_prefix, val, result)?;
            }
            Ok(())
        }
        _ => Err(format!(
            "color value for key '{interface_prefix}' must be a string or an object, got {value:?}"
        )),
    }
}

impl<'de> Deserialize<'de> for ColorTheme {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let raw: HashMap<String, Value> = HashMap::deserialize(deserializer)?;
        let mut entries = HashMap::new();

        for (key, value) in raw {
            let css_key = css_key_from(&key);
            flatten_color_value(&key, &css_key, &value, &mut entries).map_err(D::Error::custom)?;
        }

        Ok(ColorTheme { entries })
    }
}

impl ColorTheme {
    pub fn add_color(&mut self, name: &str, value: &str) {
        // The map key must own a `String` regardless; `css_key_from` borrows on
        // the common dot-free path so only the `.`-present case rebuilds.
        let css_key = css_key_from(name).into_owned();
        self.entries.insert(
            css_key.clone(),
            ColorEntry {
                interface_key: name.to_string(),
                css_key,
                value: value.to_string(),
            },
        );
    }

    /// Get all interface keys (for TypeScript interface generation, with dots)
    pub fn interface_keys(&self) -> impl Iterator<Item = &String> {
        self.entries.values().map(|e| &e.interface_key)
    }

    /// Get iterator over (`css_key`, value) pairs for CSS generation
    pub fn css_entries(&self) -> impl Iterator<Item = (&String, &String)> {
        self.entries.iter().map(|(k, e)| (k, &e.value))
    }

    /// Get value by CSS key
    #[must_use]
    pub fn get(&self, css_key: &str) -> Option<&String> {
        self.entries.get(css_key).map(|e| &e.value)
    }

    /// Check if CSS key exists
    #[must_use]
    pub fn contains_key(&self, css_key: &str) -> bool {
        self.entries.contains_key(css_key)
    }
}

pub fn deserialize_string_from_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(i64),
        Float(f64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(Some(s)),
        StringOrNumber::Number(n) => Ok(Some(n.to_string())),
        StringOrNumber::Float(n) => Ok(Some(n.to_string())),
    }
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Typography {
    pub font_family: Option<String>,
    pub font_size: Option<String>,

    #[serde(deserialize_with = "deserialize_string_from_number", default)]
    pub font_weight: Option<String>,
    #[serde(deserialize_with = "deserialize_string_from_number", default)]
    pub line_height: Option<String>,
    pub letter_spacing: Option<String>,
}
impl Typography {
    #[must_use]
    pub const fn new(
        font_family: Option<String>,
        font_size: Option<String>,
        font_weight: Option<String>,
        line_height: Option<String>,
        letter_spacing: Option<String>,
    ) -> Self {
        Self {
            font_family,
            font_size,
            font_weight,
            line_height,
            letter_spacing,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Typographies(pub Vec<Option<Typography>>);

impl From<Vec<Option<Typography>>> for Typographies {
    fn from(v: Vec<Option<Typography>>) -> Self {
        Self(v)
    }
}

/// Helper to deserialize a typography property that can be either a single value or an array
fn deserialize_typo_prop(value: &Value) -> Result<Vec<Option<String>>, String> {
    match value {
        Value::Null => Ok(vec![None]),
        Value::String(s) => Ok(vec![Some(s.clone())]),
        Value::Number(n) => Ok(vec![Some(n.to_string())]),
        Value::Array(arr) => {
            let mut result = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    Value::Null => result.push(None),
                    Value::String(s) => result.push(Some(s.clone())),
                    Value::Number(n) => result.push(Some(n.to_string())),
                    _ => return Err(format!("Invalid typography property value: {item:?}")),
                }
            }
            Ok(result)
        }
        _ => Err(format!("Invalid typography property value: {value:?}")),
    }
}

impl<'de> Deserialize<'de> for Typographies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let value = Value::deserialize(deserializer)?;

        match &value {
            // Traditional array format: [{ fontFamily: "Arial", ... }, null, { ... }]
            Value::Array(arr) => {
                let mut result = Vec::with_capacity(arr.len());
                for item in arr {
                    if item.is_null() {
                        result.push(None);
                    } else if item.is_object() {
                        let typo: Typography =
                            serde_json::from_value(item.clone()).map_err(D::Error::custom)?;
                        result.push(Some(typo));
                    } else {
                        // Non-object/null values mean this is not a valid traditional array format
                        return Err(D::Error::custom(
                            "Typography value cannot start with an array. Use object format with property-level arrays instead.",
                        ));
                    }
                }
                Ok(Self(result))
            }
            // Compact object format: { fontFamily: "Arial", fontSize: ["16px", null, "20px"], ... }
            Value::Object(obj) => {
                // Extract each property, which can be a single value or an array
                let font_family = obj
                    .get("fontFamily")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                let font_size = obj
                    .get("fontSize")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                let font_weight = obj
                    .get("fontWeight")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                let line_height = obj
                    .get("lineHeight")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                let letter_spacing = obj
                    .get("letterSpacing")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                // Find the maximum length among all properties
                let max_len = [
                    font_family.len(),
                    font_size.len(),
                    font_weight.len(),
                    line_height.len(),
                    letter_spacing.len(),
                ]
                .into_iter()
                .max()
                .unwrap_or(1);

                // Build typography for each breakpoint level
                let mut result = Vec::with_capacity(max_len);
                for i in 0..max_len {
                    let ff = font_family.get(i).cloned().unwrap_or(None);
                    let fs = font_size.get(i).cloned().unwrap_or(None);
                    let fw = font_weight.get(i).cloned().unwrap_or(None);
                    let lh = line_height.get(i).cloned().unwrap_or(None);
                    let ls = letter_spacing.get(i).cloned().unwrap_or(None);

                    // If all properties are None for this level, push None
                    if ff.is_none() && fs.is_none() && fw.is_none() && lh.is_none() && ls.is_none()
                    {
                        result.push(None);
                    } else {
                        result.push(Some(Typography {
                            font_family: ff,
                            font_size: fs,
                            font_weight: fw,
                            line_height: lh,
                            letter_spacing: ls,
                        }));
                    }
                }

                Ok(Self(result))
            }
            _ => Err(D::Error::custom(format!(
                "Typography must be an object or array, got: {value:?}"
            ))),
        }
    }
}

/// Responsive theme token values (shared by length and shadow tokens).
/// Supports:
/// - Single string: `"8px"` -> vec![Some("8px")]
/// - Single number: `4` -> vec![Some("4")]
/// - Responsive array: `["2px", null, "4px"]` -> vec![Some("2px"), None, Some("4px")]
#[derive(Serialize, Debug)]
pub struct TokenValues(pub Vec<Option<String>>);

impl<'de> Deserialize<'de> for TokenValues {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match &value {
            Value::String(s) => Ok(Self(vec![Some(s.clone())])),
            Value::Number(n) => Ok(Self(vec![Some(n.to_string())])),
            Value::Array(arr) => {
                let result = arr
                    .iter()
                    .map(|item| match item {
                        Value::Null => Ok(None),
                        Value::String(s) => Ok(Some(s.clone())),
                        Value::Number(n) => Ok(Some(n.to_string())),
                        other => {
                            let msg = format!("Invalid token value: {other:?}");
                            Err(serde::de::Error::custom(msg))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self(result))
            }
            other => Err(serde::de::Error::custom(format!(
                "Expected string, number, or array, got: {other:?}"
            ))),
        }
    }
}

/// `LengthTheme` stores named length tokens for one theme variant.
///
/// e.g., `{ "gutterMd": ["2px", "4px"], "gutterLg": "16px", "gap": 8 }`
/// Plain numbers are multiplied by 4 and suffixed with "px" (e.g., 8 → "32px").
pub type LengthTheme = BTreeMap<String, TokenValues>;

/// `ShadowTheme` stores a set of named shadow tokens for one theme variant
/// e.g., `{ "sm": "0 1px 2px rgba(0,0,0,0.1)", "md": ["0 2px 4px rgba(0,0,0,0.1)", null, "0 4px 8px rgba(0,0,0,0.2)"] }`
pub type ShadowTheme = BTreeMap<String, TokenValues>;

/// Collect, per token name, the breakpoint levels that have a value in any theme variant.
fn token_levels(
    themes: &BTreeMap<String, BTreeMap<String, TokenValues>>,
) -> BTreeMap<String, Vec<u8>> {
    // Accumulate presence per name in a `u16` bitmask over levels instead of an
    // `entry.contains(&level)` linear rescan of a growing `Vec<u8>` (bounded O(k²)
    // per token). Levels index breakpoints (realistically < 16), so a `u16` mask
    // covers every reachable level; any level >= 16 falls back to the linear probe
    // so behavior is preserved for out-of-range inputs. The ascending `Vec<u8>` is
    // materialized at the end, keeping the existing output ordering byte-identical.
    let masks = themes.values().flat_map(|theme| theme.iter()).fold(
        BTreeMap::<String, (u16, Option<Vec<u8>>)>::new(),
        |mut acc, (name, values)| {
            // Borrow-probe before allocating an owned key: only clone `name`
            // into a new entry on a genuine miss. Re-inserting the same token
            // name across every theme variant would otherwise clone ~N·K owned
            // `String` keys for only ~K distinct entries (the standard
            // "borrow-probe before owned-key insert" pattern used elsewhere in
            // this repo, e.g. `class_num_for_key`, `add_property`).
            let (mask, overflow) = match acc.get_mut(name) {
                Some(entry) => entry,
                None => acc.entry(name.clone()).or_default(),
            };
            for (idx, value) in values.0.iter().enumerate() {
                if value.is_some()
                    && let Ok(level) = u8::try_from(idx)
                {
                    if level < 16 {
                        *mask |= 1u16 << level;
                    } else {
                        // Levels >= 16 never occur for realistic themes
                        // (breakpoints < 16), so the overflow `Vec` is only
                        // allocated on first use, keeping the common case
                        // allocation-free instead of a per-token empty `Vec`.
                        //
                        // This branch is COLD by construction: every reachable
                        // theme has fewer than 16 breakpoints, so the `u16` mask
                        // above absorbs all real inputs and this `contains`-guarded
                        // O(k²) dedupe never runs in practice. The `debug_assert!`
                        // documents that expectation and would fire in dev/test if a
                        // future change ever drove a level into the overflow path,
                        // signalling the mask width (and this fallback) needs review.
                        // It compiles out entirely in release, so the hot path is
                        // unaffected.
                        debug_assert!(
                            false,
                            "overflow branch entered for level < 16, which the u16 mask should have handled"
                        );
                        let overflow = overflow.get_or_insert_with(Vec::new);
                        if !overflow.contains(&level) {
                            overflow.push(level);
                        }
                    }
                }
            }
            acc
        },
    );
    masks
        .into_iter()
        .map(|(name, (mask, overflow))| {
            // Expand only the SET bits (lowest-first) instead of scanning the
            // full 0..16 range per token, and presize the Vec exactly. Bits are
            // still visited ascending, so the output order is byte-identical.
            let overflow_len = overflow.as_ref().map_or(0, Vec::len);
            let mut levels: Vec<u8> = Vec::with_capacity(mask.count_ones() as usize + overflow_len);
            let mut m = mask;
            while m != 0 {
                levels.push(m.trailing_zeros() as u8);
                m &= m - 1;
            }
            if let Some(mut overflow) = overflow {
                overflow.sort_unstable();
                levels.extend(overflow);
            }
            (name, levels)
        })
        .collect()
}

fn default_variant_key<T>(themes: &BTreeMap<String, T>) -> Option<&str> {
    if themes.contains_key("default") {
        Some("default")
    } else if themes.contains_key("light") {
        Some("light")
    } else {
        themes.keys().next().map(String::as_str)
    }
}

/// Sort variant `(name, value)` entries so the `default_key` variant sorts first, with the
/// remaining variants ordered by name. Encoded as a bool-tuple compare (`is-not-default`, name):
/// `false < true` places the default key ahead of every other, then names order the rest.
/// This is the single authoritative definition of the "default first, then name" variant order
/// shared by the color and length/shadow CSS-variable emitters. Allocation-free.
fn sort_variants_default_first<T>(entries: &mut [(&String, T)], default_key: &str) {
    entries.sort_by(|a, b| {
        (a.0.as_str() != default_key)
            .cmp(&(b.0.as_str() != default_key))
            .then_with(|| a.0.cmp(b.0))
    });
}

/// Convert a JSON number to a length value: `n * 4` + "px".
fn number_to_length(n: &serde_json::Number) -> String {
    // as_f64() covers both integer and float JSON numbers
    let val = n.as_f64().unwrap_or(0.0) * 4.0;
    #[allow(clippy::cast_possible_truncation)]
    if val.fract() == 0.0 {
        let v = val as i64;
        format!("{v}px")
    } else {
        format!("{val}px")
    }
}

/// Deserialize a single length token value, converting numbers via `number_to_length`.
fn deserialize_length_value(value: &Value) -> Result<TokenValues, String> {
    match value {
        Value::String(s) => Ok(TokenValues(vec![Some(s.clone())])),
        Value::Number(n) => Ok(TokenValues(vec![Some(number_to_length(n))])),
        Value::Array(arr) => {
            let mut result = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    Value::Null => result.push(None),
                    Value::String(s) => result.push(Some(s.clone())),
                    Value::Number(n) => result.push(Some(number_to_length(n))),
                    _ => {
                        return Err(format!("Invalid length value in array: {item:?}"));
                    }
                }
            }
            Ok(TokenValues(result))
        }
        _ => Err(format!(
            "Length value must be a string, number, or array, got: {value:?}"
        )),
    }
}

/// Custom deserializer for the `length` field that converts plain numbers to `n*4px`.
fn deserialize_length_themes<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, LengthTheme>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::deserialize(deserializer)?;
    let mut result = BTreeMap::new();
    for (variant, tokens) in raw {
        let mut theme = BTreeMap::new();
        for (name, value) in tokens {
            let tv = deserialize_length_value(&value).map_err(serde::de::Error::custom)?;
            theme.insert(name, tv);
        }
        result.insert(variant, theme);
    }
    Ok(result)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    #[serde(default)]
    pub colors: BTreeMap<String, ColorTheme>,
    #[serde(default = "default_breakpoints")]
    pub breakpoints: Vec<u16>,
    #[serde(default)]
    pub typography: BTreeMap<String, Typographies>,
    #[serde(default, deserialize_with = "deserialize_length_themes")]
    pub length: BTreeMap<String, LengthTheme>,
    #[serde(default, alias = "shadow")]
    pub shadows: BTreeMap<String, ShadowTheme>,
}

fn default_breakpoints() -> Vec<u16> {
    vec![0, 480, 768, 992, 1280, 1600]
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: Default::default(),
            breakpoints: default_breakpoints(),
            typography: BTreeMap::new(),
            length: BTreeMap::new(),
            shadows: BTreeMap::new(),
        }
    }
}

impl Theme {
    pub fn update_breakpoints(&mut self, breakpoints: Vec<u16>) {
        for (idx, value) in breakpoints.iter().enumerate() {
            let prev = self.breakpoints.get_mut(idx);
            if let Some(prev) = prev {
                *prev = *value;
            } else {
                self.breakpoints.push(*value);
            }
        }
    }

    pub fn add_color_theme(&mut self, name: &str, theme: ColorTheme) {
        self.colors.insert(name.to_string(), theme);
    }

    pub fn add_typography(&mut self, name: &str, typography: Vec<Option<Typography>>) {
        self.typography.insert(name.to_string(), typography.into());
    }

    pub fn add_length(&mut self, variant: &str, name: &str, values: Vec<Option<String>>) {
        self.length
            .entry(variant.to_string())
            .or_default()
            .insert(name.to_string(), TokenValues(values));
    }

    pub fn add_shadow(&mut self, variant: &str, name: &str, values: Vec<Option<String>>) {
        self.shadows
            .entry(variant.to_string())
            .or_default()
            .insert(name.to_string(), TokenValues(values));
    }

    pub fn get_default_theme(&self) -> Option<String> {
        default_variant_key(&self.colors).map(str::to_string)
    }

    #[must_use]
    pub fn get_length_token_levels(&self) -> BTreeMap<String, Vec<u8>> {
        token_levels(&self.length)
    }

    #[must_use]
    pub fn get_shadow_token_levels(&self) -> BTreeMap<String, Vec<u8>> {
        token_levels(&self.shadows)
    }

    #[must_use]
    pub fn get_default_length_value(&self, token: &str) -> Option<&str> {
        let default_key = default_variant_key(&self.length)?;
        self.length
            .get(default_key)?
            .get(token)?
            .0
            .first()?
            .as_deref()
    }

    #[must_use]
    pub fn get_default_shadow_value(&self, token: &str) -> Option<&str> {
        let default_key = default_variant_key(&self.shadows)?;
        self.shadows
            .get(default_key)?
            .get(token)?
            .0
            .first()?
            .as_deref()
    }

    #[must_use]
    pub fn to_css(&self) -> String {
        // Seed a cheap lower-bound capacity so the initial 0→8→16→… grow chain
        // before the first `:root{` write is skipped. `colors.len()` is a safe
        // lower bound (at least one `:root`-ish block per variant); byte-identical.
        let mut theme_declaration = String::with_capacity(self.colors.len().saturating_mul(64));

        let default_theme_key = default_variant_key(&self.colors);
        if let Some(default_theme_key) = default_theme_key {
            let single_theme = self.colors.len() <= 1;
            // For a single (or zero) color variant the `default`-first sort is a no-op, so the
            // intermediate `Vec` collect + `sort_variants_default_first` only reproduces the
            // map's own iteration order. Skip the sort call entirely then; iterate the map
            // straight into the Vec. The multi-variant path still sorts (order matters there).
            // Byte-identical: a single-element BTreeMap yields the same lone `(name, theme)`.
            let entries: Vec<(&String, &ColorTheme)> = if single_theme {
                self.colors.iter().collect()
            } else {
                let mut col: Vec<_> = self.colors.iter().collect();
                sort_variants_default_first(&mut col, default_theme_key);
                col
            };
            // if other theme is exists, should use light-dark function
            let other_theme_key: Option<&str> = if entries.len() == 2 {
                entries
                    .iter()
                    .find(|(k, _)| **k != default_theme_key)
                    .map(|(k, _)| k.as_str())
            } else {
                None
            };
            // The default variant's optimized color values are invariant across variants, yet the
            // non-default (`theme_key.is_some()`, 3+ theme) branch re-optimizes them once per
            // variant. Precompute them once so each `optimize_value` runs a single time per color.
            // Only worth materializing when a second variant re-reads the map: for a single
            // variant each value is used exactly once, so building the intermediate `HashMap`
            // (extra hashing + one allocation) buys nothing over the default arm's inline
            // `Cow::Owned(optimize_value(value))` map-miss fallback. Skip it entirely then.
            let default_optimized_colors: HashMap<&str, String> = if single_theme {
                HashMap::new()
            } else {
                self.colors
                    .get(default_theme_key)
                    .map(|d| {
                        d.css_entries()
                            .map(|(k, v)| (k.as_str(), optimize_value(v)))
                            .collect()
                    })
                    .unwrap_or_default()
            };
            // `single_theme` is loop-invariant across every default-variant color, so decide the
            // optimized-value source ONCE here instead of re-testing the branch per color inside the
            // inner loop below. In the `single_theme` case `default_optimized_colors` is empty, so
            // the probe would always miss and fall through to `optimize_value` anyway — collapse it
            // to a direct owned optimize. Otherwise borrow the precomputed value, recomputing only on
            // the rare map miss. Emitted bytes are identical to the old per-color branch.
            let resolve_default_optimized = |prop: &str, value: &str| -> Cow<str> {
                if single_theme {
                    Cow::Owned(optimize_value(value))
                } else {
                    default_optimized_colors.get(prop).map_or_else(
                        || Cow::Owned(optimize_value(value)),
                        |v| Cow::Borrowed(v.as_str()),
                    )
                }
            };
            for (theme_name, theme_properties) in entries {
                let mut theme_contents = String::new();
                let theme_key = if *theme_name == default_theme_key {
                    None
                } else {
                    Some(theme_name)
                };
                if let Some(theme_key) = theme_key {
                    theme_declaration.push_str(":root[data-theme=");
                    theme_declaration.push_str(theme_key);
                    theme_declaration.push_str("]{");
                    push_css_declaration(&mut theme_contents, "color-scheme:dark");
                } else {
                    theme_declaration.push_str(":root{");
                    if !single_theme {
                        push_css_declaration(&mut theme_contents, "color-scheme:light");
                    }
                }
                // Non-default variants in a two-theme pair only contribute `color-scheme:dark`;
                // the default variant already emits their colors via `light-dark(...)`. Guard the
                // whole per-color pass so that partner variant skips the otherwise no-op iterator.
                if theme_key.is_none() || other_theme_key.is_none() {
                    for (prop, value) in theme_properties.css_entries() {
                        if theme_key.is_some() {
                            // The map may not contain `prop` (a color present in this variant but
                            // absent from the default), so optimize it here.
                            let optimized_value = optimize_value(value);
                            if let Some(default_value) = default_optimized_colors
                                .get(prop.as_str())
                                .and_then(|default_optimized| {
                                    if *default_optimized == optimized_value {
                                        None
                                    } else {
                                        Some(optimized_value)
                                    }
                                })
                            {
                                push_css_variable(&mut theme_contents, prop, &default_value);
                            }
                        } else {
                            // Default variant. The `single_theme`-invariant source selection is decided
                            // once by `resolve_default_optimized` (hoisted above the entries loop): for
                            // the single-variant case it optimizes directly (the map is empty, so the
                            // probe would always miss); otherwise it borrows the precomputed value keyed
                            // by `prop`, recomputing only on a rare miss (saves one `optimize_value` call
                            // and one `String` allocation per default-variant color).
                            let optimized_value = resolve_default_optimized(prop.as_str(), value);
                            let optimized_value: &str = &optimized_value;
                            let other_theme_value = other_theme_key.and_then(|other_theme_key| {
                                self.colors.get(other_theme_key).and_then(|v| {
                                    v.get(prop).and_then(|v| {
                                        let other_theme_value = optimize_value(v.as_str());
                                        if other_theme_value == optimized_value {
                                            None
                                        } else {
                                            Some(other_theme_value)
                                        }
                                    })
                                })
                            });
                            // default theme
                            if !theme_contents.is_empty() {
                                theme_contents.push(';');
                            }
                            theme_contents.push_str("--");
                            theme_contents.push_str(prop);
                            theme_contents.push(':');
                            if let Some(other_theme_value) = other_theme_value {
                                theme_contents.push_str("light-dark(");
                                theme_contents.push_str(optimized_value);
                                theme_contents.push(',');
                                theme_contents.push_str(&other_theme_value);
                                theme_contents.push(')');
                            } else {
                                theme_contents.push_str(optimized_value);
                            }
                        }
                    }
                }
                theme_declaration.push_str(&theme_contents);
                theme_declaration.push('}');
            }
        }
        let mut css = theme_declaration;
        let mut level_map = BTreeMap::<u8, String>::new();
        // Reuse a single buffer across every typography entry×level: `clear()` keeps the
        // backing capacity alive so populated levels amortize to ~1 allocation total
        // instead of one allocate→grow→copy→free cycle per non-empty level.
        let mut css_content = String::new();
        // Loop-invariant: `resolve_into` captures no loop variable (it only calls the free fn
        // `optimize_value`), so define it once instead of reconstructing the closure on every
        // typography entry×level iteration. For the `$token` path it writes `var(--token)`
        // byte-for-byte straight into the target buffer, avoiding the throwaway `format!`
        // `String` the old `resolve` allocated per themed property; only the non-`$` path
        // still needs `optimize_value`'s owned `String`.
        let resolve_into = |out: &mut String, v: &str| {
            if let Some(token) = v.strip_prefix('$') {
                out.push_str("var(--");
                out.push_str(token);
                out.push(')');
            } else {
                out.push_str(&optimize_value(v));
            }
        };
        for ty in &self.typography {
            for (idx, t) in ty.1.0.iter().enumerate() {
                if let Some(t) = t {
                    css_content.clear();
                    push_typography_property(
                        &mut css_content,
                        "font-family",
                        t.font_family.as_deref(),
                        &resolve_into,
                    );
                    push_typography_property(
                        &mut css_content,
                        "font-size",
                        t.font_size.as_deref(),
                        &resolve_into,
                    );
                    push_typography_property(
                        &mut css_content,
                        "font-weight",
                        t.font_weight.as_deref(),
                        &resolve_into,
                    );
                    push_typography_property(
                        &mut css_content,
                        "line-height",
                        t.line_height.as_deref(),
                        &resolve_into,
                    );
                    push_typography_property(
                        &mut css_content,
                        "letter-spacing",
                        t.letter_spacing.as_deref(),
                        &resolve_into,
                    );

                    if !css_content.is_empty() {
                        let level_css = level_map.entry(idx as u8).or_default();
                        level_css.push_str(".typo-");
                        level_css.push_str(ty.0);
                        level_css.push('{');
                        level_css.push_str(&css_content);
                        level_css.push('}');
                    }
                }
            }
        }
        for (level, css_vec) in level_map {
            if level == 0 {
                css.push_str(&css_vec);
            } else if let Some(bp) = self.breakpoints.get(level as usize) {
                write!(css, "@media(min-width:{bp}px)")
                    .unwrap_or_else(|err| panic!("failed to write CSS into string: {err}"));
                css.push('{');
                css.push_str(&css_vec);
                css.push('}');
            }
        }
        // Generate CSS variables for length tokens
        Self::write_themed_css_vars(&mut css, &self.length, &self.breakpoints);
        // Generate CSS variables for shadow tokens
        Self::write_themed_css_vars(&mut css, &self.shadows, &self.breakpoints);
        css
    }

    /// Shared helper: generates CSS custom properties from themed token maps.
    /// Used by both length and shadow tokens (and any future token types with the same shape).
    fn write_themed_css_vars(
        css: &mut String,
        themes: &BTreeMap<String, BTreeMap<String, TokenValues>>,
        breakpoints: &[u16],
    ) {
        let Some(default_key) = default_variant_key(themes) else {
            return;
        };

        // Sort variants: default first, then alphabetical.
        // For a single (or zero) variant the `default`-first sort is a no-op that only
        // reproduces the map's own iteration order (a 1-element BTreeMap yields the same lone
        // entry). Skip the comparator setup + `sort_by` then, mirroring the color path's
        // `single_theme` guard in `to_css` (and the `sorted_variants.len() <= 1` guard just
        // below). Byte-identical output; the multi-variant path still sorts (order matters there).
        let mut sorted_variants: Vec<_> = themes.iter().collect();
        if sorted_variants.len() > 1 {
            sort_variants_default_first(&mut sorted_variants, default_key);
        }

        let default_theme = themes.get(default_key);

        // The default variant's optimized token values are invariant across variants, yet the
        // `is_same_as_default` check below re-optimizes them once per non-default variant.
        // Precompute them once so each `optimize_value` on a default value runs a single time.
        // The map is read only inside `is_same_as_default` (`!is_default && …`), which is never
        // true when there is a single variant — mirroring the color path's `single_theme` guard,
        // skip building it entirely then so its `optimize_value` calls and owned `String`s (one
        // per default token value) are not allocated just to be discarded.
        let default_optimized: HashMap<(&str, usize), String> = if sorted_variants.len() <= 1 {
            HashMap::new()
        } else {
            default_theme
                .map(|dt| {
                    dt.iter()
                        .flat_map(|(name, values)| {
                            values.0.iter().enumerate().filter_map(move |(idx, dval)| {
                                dval.as_ref()
                                    .map(|d| ((name.as_str(), idx), optimize_value(d)))
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        for (variant_name, token_theme) in &sorted_variants {
            let is_default = *variant_name == default_key;
            // Write the `:root` / `:root[data-theme=<name>]` prefix directly into
            // `css` at each use site instead of allocating one owned `String` per
            // variant. Emitted bytes are identical to the former `format!`.
            let write_selector = |css: &mut String| {
                css.push_str(":root");
                if !is_default {
                    css.push_str("[data-theme=");
                    css.push_str(variant_name);
                    css.push(']');
                }
            };

            // Group variables by breakpoint level without allocating one String per variable.
            let mut level_map = BTreeMap::<usize, String>::new();
            for (name, values) in *token_theme {
                // `name` is invariant across the `idx` iteration, so borrow it as `&str`
                // once here instead of re-`as_str()`ing it inside every value probe/push.
                let name_str = name.as_str();
                for (idx, val) in values.0.iter().enumerate() {
                    if let Some(v) = val {
                        let optimized = optimize_value(v);
                        let is_same_as_default = !is_default
                            && default_optimized
                                .get(&(name_str, idx))
                                .is_some_and(|d| *d == optimized);
                        if !is_same_as_default {
                            let vars = level_map.entry(idx).or_default();
                            if !vars.is_empty() {
                                vars.push(';');
                            }
                            vars.push_str("--");
                            vars.push_str(name_str);
                            vars.push(':');
                            vars.push_str(&optimized);
                        }
                    }
                }
            }

            for (level, vars) in &level_map {
                if !vars.is_empty() {
                    if *level == 0 {
                        write_selector(css);
                        css.push('{');
                        css.push_str(vars);
                        css.push('}');
                    } else if let Some(bp) = breakpoints.get(*level) {
                        write!(css, "@media(min-width:{bp}px){{")
                            .unwrap_or_else(|err| panic!("failed to write CSS into string: {err}"));
                        write_selector(css);
                        css.push('{');
                        css.push_str(vars);
                        css.push_str("}}");
                    }
                }
            }
        }
    }
}

fn push_typography_property(
    css_content: &mut String,
    property: &str,
    value: Option<&str>,
    resolve_into: &impl Fn(&mut String, &str),
) {
    let Some(value) = value else {
        return;
    };
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    if !css_content.is_empty() {
        css_content.push(';');
    }
    css_content.push_str(property);
    css_content.push(':');
    resolve_into(css_content, value);
}

fn push_css_declaration(css_content: &mut String, declaration: &str) {
    if !css_content.is_empty() {
        css_content.push(';');
    }
    css_content.push_str(declaration);
}

fn push_css_variable(css_content: &mut String, name: &str, value: &str) {
    if !css_content.is_empty() {
        css_content.push(';');
    }
    css_content.push_str("--");
    css_content.push_str(name);
    css_content.push(':');
    css_content.push_str(value);
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;

    fn make_named_color_theme(name: &str, value: &str) -> ColorTheme {
        let mut ct = ColorTheme::default();
        ct.add_color(name, value);
        ct
    }

    #[test]
    fn to_css_from_theme() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");

        assert_eq!(color_theme.css_entries().count(), 1);

        theme.add_color_theme("default", color_theme);
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#fff");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "default",
            vec![
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("16px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );

        theme.add_typography(
            "default1",
            vec![
                None,
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );
        let css = theme.to_css();
        assert_debug_snapshot!(css);

        assert_eq!(Theme::default().to_css(), "");
        let mut theme = Theme::default();
        theme.add_typography(
            "default",
            vec![Some(Typography::new(None, None, None, None, None))],
        );
        assert_eq!(theme.to_css(), "");

        let mut theme = Theme::default();
        theme.add_color_theme("default", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#001"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#002"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());
    }

    #[rstest]
    #[case(
        vec![0, 480, 768, 992, 1280],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600, 1920],
        vec![0, 480, 768, 992, 1280, 1600, 1920]
    )]
    fn update_breakpoints(#[case] input: Vec<u16>, #[case] expected: Vec<u16>) {
        let mut theme = Theme::default();
        theme.update_breakpoints(input);
        assert_eq!(theme.breakpoints, expected);
    }

    #[test]
    fn test_nested_color_theme_deserialization() {
        // Test simple string values
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "primary": "#000"
                    }
                }
            }"##,
        )
        .unwrap();
        assert!(theme.colors.get("light").unwrap().contains_key("primary"));
        assert_eq!(
            theme.colors.get("light").unwrap().get("primary").unwrap(),
            "#000"
        );

        // Test dot notation keys (e.g., "primary.100" -> "primary-100")
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "primary.100": "#100",
                        "primary.200": "#200"
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();
        assert!(light.contains_key("primary-100"));
        assert!(light.contains_key("primary-200"));
        assert_eq!(light.get("primary-100").unwrap(), "#100");
        assert_eq!(light.get("primary-200").unwrap(), "#200");

        // Test nested object (e.g., "hello": { "100": "#000" } -> "hello-100")
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "hello": {
                            "100": "#100",
                            "200": "#200"
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();
        assert!(light.contains_key("hello-100"));
        assert!(light.contains_key("hello-200"));
        assert_eq!(light.get("hello-100").unwrap(), "#100");
        assert_eq!(light.get("hello-200").unwrap(), "#200");

        // Test mixed: simple, dot notation, and nested
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "primary": "#000",
                        "secondary.100": "#sec100",
                        "gray": {
                            "50": "#gray50",
                            "100": "#gray100"
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();
        assert_eq!(light.get("primary").unwrap(), "#000");
        assert_eq!(light.get("secondary-100").unwrap(), "#sec100");
        assert_eq!(light.get("gray-50").unwrap(), "#gray50");
        assert_eq!(light.get("gray-100").unwrap(), "#gray100");
    }

    #[test]
    fn test_nested_color_theme_to_css() {
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "primary": "#000",
                        "gray": {
                            "100": "#f5f5f5",
                            "200": "#eee"
                        }
                    },
                    "dark": {
                        "primary": "#fff",
                        "gray": {
                            "100": "#333",
                            "200": "#444"
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let css = theme.to_css();
        // Should contain CSS variables for flattened keys
        assert!(css.contains("--primary:"));
        assert!(css.contains("--gray-100:"));
        assert!(css.contains("--gray-200:"));
        // Check light-dark() function is used for color switching
        assert!(css.contains("light-dark(#000,#FFF)") || css.contains("light-dark(#000,#fff)"));
        assert!(css.contains("color-scheme:light"));
        assert!(css.contains("color-scheme:dark"));
    }

    #[test]
    fn test_add_color_with_dot_notation() {
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary.100", "#100");
        color_theme.add_color("primary.200", "#200");

        // CSS keys should have dashes instead of dots
        assert!(color_theme.contains_key("primary-100"));
        assert!(color_theme.contains_key("primary-200"));
        assert!(!color_theme.contains_key("primary.100"));
    }

    #[test]
    fn test_deep_nested_color_should_succeed() {
        // Deep nesting should be flattened with dashes
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "primary": {
                            "100": {
                                "light": "#f0f",
                                "dark": "#0f0"
                            },
                            "200": "#200"
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();
        // primary -> 100 -> light = "primary-100-light"
        assert!(light.contains_key("primary-100-light"));
        assert!(light.contains_key("primary-100-dark"));
        assert!(light.contains_key("primary-200"));
        assert_eq!(light.get("primary-100-light").unwrap(), "#f0f");
        assert_eq!(light.get("primary-100-dark").unwrap(), "#0f0");
        assert_eq!(light.get("primary-200").unwrap(), "#200");
    }

    #[test]
    fn test_very_deep_nested_color() {
        // 4 levels deep: a -> b -> c -> d -> value
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "a": {
                            "b": {
                                "c": {
                                    "d": "#deep"
                                }
                            }
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();
        assert!(light.contains_key("a-b-c-d"));
        assert_eq!(light.get("a-b-c-d").unwrap(), "#deep");
    }

    #[test]
    fn test_nested_with_number_value_should_fail() {
        // Nested object with non-string value should fail
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "colors": {
                    "light": {
                        "gray": {
                            "100": 123
                        }
                    }
                }
            }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_interface_keys_vs_css_keys() {
        // interface_keys should preserve dots, css_keys should use dashes
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "gray": {
                            "100": "#f5f5f5",
                            "200": "#eee"
                        },
                        "primary.light": "#000"
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();

        // Collect interface keys
        let interface_keys: Vec<_> = light.interface_keys().cloned().collect();
        // Collect CSS keys
        let css_keys: Vec<_> = light.css_entries().map(|(k, _)| k.clone()).collect();

        // Interface keys should use dots for nested objects
        assert!(interface_keys.contains(&"gray.100".to_string()));
        assert!(interface_keys.contains(&"gray.200".to_string()));
        // Dot notation in original key stays as is
        assert!(interface_keys.contains(&"primary.light".to_string()));

        // CSS keys should use dashes
        assert!(css_keys.contains(&"gray-100".to_string()));
        assert!(css_keys.contains(&"gray-200".to_string()));
        assert!(css_keys.contains(&"primary-light".to_string()));
    }

    #[test]
    fn test_deep_nested_interface_keys() {
        let theme: Theme = serde_json::from_str(
            r##"{
                "colors": {
                    "light": {
                        "a": {
                            "b": {
                                "c": "#deep"
                            }
                        }
                    }
                }
            }"##,
        )
        .unwrap();
        let light = theme.colors.get("light").unwrap();

        // Interface key uses dots
        assert!(light.interface_keys().any(|key| key == "a.b.c"));
        // CSS key uses dashes
        assert!(light.css_entries().any(|(key, _)| key == "a-b-c"));
    }

    #[test]
    fn test_compact_typography_format() {
        // Test new compact format with property-level arrays
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontFamily": "Pretendard",
                        "fontStyle": "normal",
                        "fontWeight": 800,
                        "fontSize": ["38px", null, null, null, "52px"],
                        "lineHeight": 1.3,
                        "letterSpacing": "-0.03em"
                    }
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(h1.0.len(), 5);

        // First breakpoint
        let first = h1.0[0].as_ref().unwrap();
        assert_eq!(first.font_family, Some("Pretendard".to_string()));
        assert_eq!(first.font_size, Some("38px".to_string()));
        assert_eq!(first.font_weight, Some("800".to_string()));
        assert_eq!(first.line_height, Some("1.3".to_string()));
        assert_eq!(first.letter_spacing, Some("-0.03em".to_string()));

        // Middle breakpoints should be None (all properties are single values except fontSize)
        assert!(h1.0[1].is_none());
        assert!(h1.0[2].is_none());
        assert!(h1.0[3].is_none());

        // Last breakpoint (only fontSize changes)
        let last = h1.0[4].as_ref().unwrap();
        assert_eq!(last.font_size, Some("52px".to_string()));
    }

    #[test]
    fn test_compact_typography_all_arrays() {
        // Test compact format where multiple properties have arrays
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "body": {
                        "fontFamily": "Pretendard",
                        "fontSize": ["14px", null, "16px"],
                        "fontWeight": [500, null, 600],
                        "lineHeight": [1.3, null, 1.5]
                    }
                }
            }"#,
        )
        .unwrap();

        let body = theme.typography.get("body").unwrap();
        assert_eq!(body.0.len(), 3);

        // First breakpoint
        let first = body.0[0].as_ref().unwrap();
        assert_eq!(first.font_family, Some("Pretendard".to_string()));
        assert_eq!(first.font_size, Some("14px".to_string()));
        assert_eq!(first.font_weight, Some("500".to_string()));
        assert_eq!(first.line_height, Some("1.3".to_string()));

        // Middle is None
        assert!(body.0[1].is_none());

        // Third breakpoint
        let third = body.0[2].as_ref().unwrap();
        assert_eq!(third.font_size, Some("16px".to_string()));
        assert_eq!(third.font_weight, Some("600".to_string()));
        assert_eq!(third.line_height, Some("1.5".to_string()));
    }

    #[test]
    fn test_compact_typography_single_value() {
        // Test compact format with all single values (no arrays)
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "caption": {
                        "fontFamily": "Pretendard",
                        "fontStyle": "normal",
                        "fontWeight": 500,
                        "fontSize": "14px",
                        "lineHeight": 1.4,
                        "letterSpacing": "-0.03em"
                    }
                }
            }"#,
        )
        .unwrap();

        let caption = theme.typography.get("caption").unwrap();
        assert_eq!(caption.0.len(), 1);

        let first = caption.0[0].as_ref().unwrap();
        assert_eq!(first.font_family, Some("Pretendard".to_string()));
        assert_eq!(first.font_size, Some("14px".to_string()));
        assert_eq!(first.font_weight, Some("500".to_string()));
        assert_eq!(first.line_height, Some("1.4".to_string()));
        assert_eq!(first.letter_spacing, Some("-0.03em".to_string()));
    }

    #[test]
    fn test_traditional_typography_array_still_works() {
        // Ensure backward compatibility with traditional array format
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": [
                        {
                            "fontFamily": "Pretendard",
                            "fontWeight": 800,
                            "fontSize": "38px",
                            "lineHeight": 1.3
                        },
                        null,
                        null,
                        null,
                        {
                            "fontFamily": "Pretendard",
                            "fontWeight": 800,
                            "fontSize": "52px",
                            "lineHeight": 1.3
                        }
                    ]
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(h1.0.len(), 5);

        let first = h1.0[0].as_ref().unwrap();
        assert_eq!(first.font_size, Some("38px".to_string()));

        assert!(h1.0[1].is_none());
        assert!(h1.0[2].is_none());
        assert!(h1.0[3].is_none());

        let last = h1.0[4].as_ref().unwrap();
        assert_eq!(last.font_size, Some("52px".to_string()));
    }

    #[test]
    fn test_compact_typography_css_output() {
        // Verify CSS output is correct for compact format
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontFamily": "Pretendard",
                        "fontSize": ["38px", null, null, null, "52px"],
                        "fontWeight": 800,
                        "lineHeight": 1.3
                    }
                }
            }"#,
        )
        .unwrap();

        let css = theme.to_css();
        // Should have base style
        assert!(css.contains(".typo-h1{"));
        assert!(css.contains("font-family:Pretendard"));
        assert!(css.contains("font-size:38px"));
        assert!(css.contains("font-weight:800"));
        // Should have media query for breakpoint 4 (1280px)
        assert!(css.contains("@media(min-width:1280px)"));
        assert!(css.contains("font-size:52px"));
    }

    #[test]
    fn test_invalid_top_level_array_should_fail() {
        // Top-level array that's not traditional format should fail
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": ["38px", null, "52px"]
                }
            }"#,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    #[test]
    fn test_typography_variable_reference() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "body": {
                        "fontSize": "$text",
                        "lineHeight": "$leading",
                        "fontWeight": 400
                    }
                }
            }"#,
        )
        .unwrap();

        let css = theme.to_css();
        assert!(
            css.contains("font-size:var(--text)"),
            "Expected font-size:var(--text), got: {css}"
        );
        assert!(
            css.contains("line-height:var(--leading)"),
            "Expected line-height:var(--leading), got: {css}"
        );
        assert!(css.contains("font-weight:400"));
    }

    #[test]
    fn test_typography_variable_reference_responsive() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "heading": [
                        {
                            "fontSize": "$textSm",
                            "fontWeight": 700
                        },
                        null,
                        null,
                        null,
                        {
                            "fontSize": "$textLg",
                            "fontWeight": 700
                        }
                    ]
                }
            }"#,
        )
        .unwrap();

        let css = theme.to_css();
        assert!(
            css.contains("font-size:var(--textSm)"),
            "Expected font-size:var(--textSm), got: {css}"
        );
        assert!(
            css.contains("font-size:var(--textLg)"),
            "Expected font-size:var(--textLg), got: {css}"
        );
    }

    #[test]
    fn test_mixed_typography_formats() {
        // Test that both formats can coexist in the same theme
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": [
                        { "fontFamily": "Pretendard", "fontSize": "38px" },
                        null,
                        { "fontFamily": "Pretendard", "fontSize": "52px" }
                    ],
                    "body": {
                        "fontFamily": "Pretendard",
                        "fontSize": ["14px", null, "16px"]
                    }
                }
            }"#,
        )
        .unwrap();

        // Traditional format
        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(h1.0.len(), 3);
        assert_eq!(
            h1.0[0].as_ref().unwrap().font_size,
            Some("38px".to_string())
        );

        // Compact format
        let body = theme.typography.get("body").unwrap();
        assert_eq!(body.0.len(), 3);
        assert_eq!(
            body.0[0].as_ref().unwrap().font_size,
            Some("14px".to_string())
        );
    }

    #[test]
    fn test_deserialize_typo_prop_null_value() {
        // Test compact format with null values in arrays
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontFamily": null,
                        "fontSize": ["14px", null, "16px"]
                    }
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(h1.0.len(), 3);
        // fontFamily is null at all levels
        assert!(h1.0[0].as_ref().unwrap().font_family.is_none());
        assert_eq!(
            h1.0[0].as_ref().unwrap().font_size,
            Some("14px".to_string())
        );
    }

    #[test]
    fn test_deserialize_typo_prop_invalid_array_value() {
        // Test that invalid values in typography arrays fail
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontSize": ["14px", {"invalid": "object"}, "16px"]
                    }
                }
            }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_typo_prop_invalid_single_value() {
        // Test that invalid single value fails
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontSize": true
                    }
                }
            }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_typography_invalid_type() {
        // Test that typography with invalid type (string) fails
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": "invalid string"
                }
            }"#,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("must be an object or array"));
    }

    #[test]
    fn test_get_default_theme_priority() {
        fn make_color_theme() -> ColorTheme {
            let mut ct = ColorTheme::default();
            ct.add_color("primary", "#000");
            ct
        }

        // Test "default" theme has highest priority
        let mut theme = Theme::default();
        theme.add_color_theme("default", make_color_theme());
        theme.add_color_theme("light", make_color_theme());
        theme.add_color_theme("dark", make_color_theme());
        assert_eq!(theme.get_default_theme(), Some("default".to_string()));

        // Test "light" theme has second priority when "default" is absent
        let mut theme = Theme::default();
        theme.add_color_theme("light", make_color_theme());
        theme.add_color_theme("dark", make_color_theme());
        theme.add_color_theme("custom", make_color_theme());
        assert_eq!(theme.get_default_theme(), Some("light".to_string()));

        // Test first theme when neither "default" nor "light" exists
        let mut theme = Theme::default();
        theme.add_color_theme("dark", make_color_theme());
        theme.add_color_theme("custom", make_color_theme());
        // BTreeMap returns keys in alphabetical order, so "custom" comes first
        assert_eq!(theme.get_default_theme(), Some("custom".to_string()));

        // Test None when no color themes exist
        let theme = Theme::default();
        assert_eq!(theme.get_default_theme(), None);
    }

    #[test]
    fn test_css_entries_iterator() {
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        color_theme.add_color("secondary.100", "#111");
        color_theme.add_color("gray.200", "#222");

        let entries: Vec<_> = color_theme.css_entries().collect();
        assert_eq!(entries.len(), 3);

        // Verify we can find all entries
        assert!(entries.iter().any(|(k, v)| *k == "primary" && *v == "#000"));
        assert!(
            entries
                .iter()
                .any(|(k, v)| *k == "secondary-100" && *v == "#111")
        );
        assert!(
            entries
                .iter()
                .any(|(k, v)| *k == "gray-200" && *v == "#222")
        );
    }

    #[test]
    fn test_typography_empty_properties_all_none() {
        // Test that empty compact format with no properties creates None
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "empty": {}
                }
            }"#,
        )
        .unwrap();

        let empty = theme.typography.get("empty").unwrap();
        assert_eq!(empty.0.len(), 1);
        assert!(empty.0[0].is_none());
    }

    #[test]
    fn test_typography_with_only_letter_spacing() {
        // Test typography with only letterSpacing property
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "letterSpacing": ["-0.02em", null, "-0.03em"]
                    }
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(h1.0.len(), 3);
        assert_eq!(
            h1.0[0].as_ref().unwrap().letter_spacing,
            Some("-0.02em".to_string())
        );
        assert!(h1.0[1].is_none());
        assert_eq!(
            h1.0[2].as_ref().unwrap().letter_spacing,
            Some("-0.03em".to_string())
        );
    }

    #[test]
    fn test_color_theme_empty() {
        let color_theme = ColorTheme::default();
        assert_eq!(color_theme.css_entries().count(), 0);
        assert_eq!(color_theme.interface_keys().count(), 0);
        assert_eq!(color_theme.css_entries().count(), 0);
        assert!(!color_theme.contains_key("any"));
        assert!(color_theme.get("any").is_none());
    }

    #[test]
    fn test_traditional_typography_with_invalid_item() {
        // Test that traditional array with invalid item (not object/null) fails
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": [
                        { "fontFamily": "Arial" },
                        "invalid string item",
                        null
                    ]
                }
            }"#,
        );
        // This should fail because "invalid string item" is not null or object
        // But the current implementation detects this as non-traditional and fails differently
        assert!(result.is_err());
    }

    #[test]
    fn test_compact_typography_different_array_lengths() {
        // Test when different properties have different array lengths
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "fontSize": ["14px", "16px"],
                        "fontWeight": ["400", "500", "600", "700"]
                    }
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        // Should use max length (4)
        assert_eq!(h1.0.len(), 4);

        // First two should have both properties
        assert_eq!(
            h1.0[0].as_ref().unwrap().font_size,
            Some("14px".to_string())
        );
        assert_eq!(
            h1.0[0].as_ref().unwrap().font_weight,
            Some("400".to_string())
        );

        assert_eq!(
            h1.0[1].as_ref().unwrap().font_size,
            Some("16px".to_string())
        );
        assert_eq!(
            h1.0[1].as_ref().unwrap().font_weight,
            Some("500".to_string())
        );

        // Last two should only have fontWeight (fontSize array is shorter)
        assert!(h1.0[2].as_ref().unwrap().font_size.is_none());
        assert_eq!(
            h1.0[2].as_ref().unwrap().font_weight,
            Some("600".to_string())
        );

        assert!(h1.0[3].as_ref().unwrap().font_size.is_none());
        assert_eq!(
            h1.0[3].as_ref().unwrap().font_weight,
            Some("700".to_string())
        );
    }

    #[test]
    fn test_typography_float_values() {
        // Test that float values are properly converted
        let theme: Theme = serde_json::from_str(
            r#"{
                "typography": {
                    "h1": {
                        "lineHeight": [1.2, 1.5, 1.8],
                        "fontWeight": [400.5, 500, 600]
                    }
                }
            }"#,
        )
        .unwrap();

        let h1 = theme.typography.get("h1").unwrap();
        assert_eq!(
            h1.0[0].as_ref().unwrap().line_height,
            Some("1.2".to_string())
        );
        assert_eq!(
            h1.0[0].as_ref().unwrap().font_weight,
            Some("400.5".to_string())
        );
    }

    #[test]
    fn test_typographies_direct_traditional_array_deserialize() {
        // Directly deserialize Typographies to ensure Value::Object branch is covered (line 183)
        let typographies: Typographies = serde_json::from_str(
            r#"[
                { "fontFamily": "Arial", "fontSize": "16px" },
                null,
                { "fontFamily": "Helvetica", "fontSize": "18px" }
            ]"#,
        )
        .unwrap();

        assert_eq!(typographies.0.len(), 3);
        assert_eq!(
            typographies.0[0].as_ref().unwrap().font_family,
            Some("Arial".to_string())
        );
        assert!(typographies.0[1].is_none());
        assert_eq!(
            typographies.0[2].as_ref().unwrap().font_family,
            Some("Helvetica".to_string())
        );
    }

    #[test]
    fn test_typographies_direct_invalid_array_item() {
        // Directly deserialize Typographies with invalid array item to cover line 188
        let result: Result<Typographies, _> = serde_json::from_str(
            r#"[
                { "fontFamily": "Arial" },
                "invalid string",
                null
            ]"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    #[test]
    fn test_typographies_direct_number_in_array() {
        // Test with number in traditional array to ensure error branch is hit
        let result: Result<Typographies, _> = serde_json::from_str(
            r#"[
                { "fontFamily": "Arial" },
                123,
                null
            ]"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    #[test]
    fn test_typographies_direct_bool_in_array() {
        // Test with boolean in traditional array
        let result: Result<Typographies, _> = serde_json::from_str(
            r#"[
                null,
                { "fontFamily": "Arial" },
                true
            ]"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    #[test]
    fn test_typographies_direct_nested_array_in_array() {
        // Test with nested array in traditional array
        let result: Result<Typographies, _> = serde_json::from_str(
            r#"[
                { "fontFamily": "Arial" },
                ["nested", "array"],
                null
            ]"#,
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    // ===== Length token tests =====

    #[test]
    fn test_length_deserialization_single_string() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gutterMd": "8px"
                    }
                }
            }"#,
        )
        .unwrap();

        let default_length = theme.length.get("default").unwrap();
        let gutter = default_length.get("gutterMd").unwrap();
        assert_eq!(gutter.0.len(), 1);
        assert_eq!(gutter.0[0], Some("8px".to_string()));
    }

    #[test]
    fn test_length_deserialization_single_number() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gap": 4
                    }
                }
            }"#,
        )
        .unwrap();

        let default_length = theme.length.get("default").unwrap();
        let gap = default_length.get("gap").unwrap();
        assert_eq!(gap.0.len(), 1);
        assert_eq!(gap.0[0], Some("16px".to_string()));
    }

    #[test]
    fn test_length_deserialization_responsive_array() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gutterMd": ["2px", "4px"]
                    }
                }
            }"#,
        )
        .unwrap();

        let default_length = theme.length.get("default").unwrap();
        let gutter = default_length.get("gutterMd").unwrap();
        assert_eq!(gutter.0.len(), 2);
        assert_eq!(gutter.0[0], Some("2px".to_string()));
        assert_eq!(gutter.0[1], Some("4px".to_string()));
    }

    #[test]
    fn test_length_deserialization_responsive_array_with_nulls() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gutterLg": ["8px", null, null, null, "16px"]
                    }
                }
            }"#,
        )
        .unwrap();

        let default_length = theme.length.get("default").unwrap();
        let gutter = default_length.get("gutterLg").unwrap();
        assert_eq!(gutter.0.len(), 5);
        assert_eq!(gutter.0[0], Some("8px".to_string()));
        assert!(gutter.0[1].is_none());
        assert!(gutter.0[2].is_none());
        assert!(gutter.0[3].is_none());
        assert_eq!(gutter.0[4], Some("16px".to_string()));
    }

    #[test]
    fn test_length_deserialization_number_in_array() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gap": [4, null, 8]
                    }
                }
            }"#,
        )
        .unwrap();

        let default_length = theme.length.get("default").unwrap();
        let gap = default_length.get("gap").unwrap();
        assert_eq!(gap.0.len(), 3);
        assert_eq!(gap.0[0], Some("16px".to_string()));
        assert!(gap.0[1].is_none());
        assert_eq!(gap.0[2], Some("32px".to_string()));
    }

    #[test]
    fn test_length_deserialization_invalid_value() {
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gap": true
                    }
                }
            }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_length_deserialization_invalid_array_value() {
        let result: Result<Theme, _> = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gap": [true]
                    }
                }
            }"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_length_css_generation_single_value() {
        let mut theme = Theme::default();
        theme.add_length("default", "gutterMd", vec![Some("8px".to_string())]);

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_generation_responsive() {
        let mut theme = Theme::default();
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_generation_responsive_with_nulls() {
        let mut theme = Theme::default();
        theme.add_length(
            "default",
            "gutterLg",
            vec![
                Some("8px".to_string()),
                None,
                None,
                None,
                Some("16px".to_string()),
            ],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_generation_multiple_tokens() {
        let mut theme = Theme::default();
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );
        theme.add_length(
            "default",
            "gutterLg",
            vec![Some("8px".to_string()), Some("16px".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_generation_with_theme_variants() {
        let mut theme = Theme::default();
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );
        theme.add_length(
            "dark",
            "gutterMd",
            vec![Some("4px".to_string()), Some("8px".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_generation_variant_skips_same_values() {
        let mut theme = Theme::default();
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );
        // Dark variant has same base value as default, different responsive
        theme.add_length(
            "dark",
            "gutterMd",
            vec![Some("2px".to_string()), Some("8px".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_css_with_colors_and_typography() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("default", color_theme);
        theme.add_typography(
            "heading",
            vec![Some(Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                None,
                None,
                None,
            ))],
        );
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_deserialization_from_json() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "length": {
                    "default": {
                        "gutterMd": ["2px", "4px"],
                        "gutterLg": "16px",
                        "gap": 8
                    }
                }
            }"#,
        )
        .unwrap();

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_length_add_length_helper() {
        let mut theme = Theme::default();
        theme.add_length("default", "sm", vec![Some("4px".to_string())]);
        theme.add_length("default", "md", vec![Some("8px".to_string())]);

        let default_length = theme.length.get("default").unwrap();
        assert_eq!(default_length.len(), 2);
        assert!(default_length.contains_key("sm"));
        assert!(default_length.contains_key("md"));
    }

    // ===== Shadow token tests =====

    #[test]
    fn test_shadow_deserialization_from_json() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "shadows": {
                    "default": {
                        "sm": "0 1px 2px rgba(0,0,0,0.1)",
                        "md": ["0 2px 4px rgba(0,0,0,0.1)", null, "0 4px 8px rgba(0,0,0,0.2)"]
                    }
                }
            }"#,
        )
        .unwrap();

        let default_shadows = theme.shadows.get("default").unwrap();
        assert_eq!(default_shadows.len(), 2);
        assert_eq!(
            default_shadows.get("sm").unwrap().0,
            vec![Some("0 1px 2px rgba(0,0,0,0.1)".to_string())]
        );
        assert_eq!(default_shadows.get("md").unwrap().0.len(), 3);
    }

    #[test]
    fn test_shadow_css_generation_single() {
        let mut theme = Theme::default();
        theme.add_shadow(
            "default",
            "sm",
            vec![Some("0 1px 2px rgba(0,0,0,.1)".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_shadow_css_generation_responsive() {
        let mut theme = Theme::default();
        theme.add_shadow(
            "default",
            "md",
            vec![
                Some("0 2px 4px rgba(0,0,0,.1)".to_string()),
                Some("0 4px 8px rgba(0,0,0,.2)".to_string()),
            ],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_shadow_css_generation_with_theme_variants() {
        let mut theme = Theme::default();
        theme.add_shadow(
            "default",
            "sm",
            vec![Some("0 1px 2px rgba(0,0,0,.1)".to_string())],
        );
        theme.add_shadow(
            "dark",
            "sm",
            vec![Some("0 1px 2px rgba(255,255,255,.1)".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_shadow_css_with_length_and_colors() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("default", color_theme);
        theme.add_length(
            "default",
            "gutterMd",
            vec![Some("2px".to_string()), Some("4px".to_string())],
        );
        theme.add_shadow(
            "default",
            "sm",
            vec![Some("0 1px 2px rgba(0,0,0,.1)".to_string())],
        );

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    // ===== Coverage: TokenValues deserialization edge cases =====

    #[test]
    fn test_token_values_deserialize_number() {
        // Covers TokenValues::deserialize Number branch (used by shadows)
        let tv: TokenValues = serde_json::from_str("42").unwrap();
        assert_eq!(tv.0, vec![Some("42".to_string())]);
    }

    #[test]
    fn test_token_values_deserialize_array_with_number() {
        // Covers array Number branch in TokenValues::deserialize
        let tv: TokenValues = serde_json::from_str(r#"["a", 10, null]"#).unwrap();
        assert_eq!(
            tv.0,
            vec![Some("a".to_string()), Some("10".to_string()), None]
        );
    }

    #[test]
    fn test_token_values_deserialize_invalid_array_item() {
        // Covers _ branch inside array match
        let result: Result<TokenValues, _> = serde_json::from_str(r"[true]");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_values_deserialize_invalid_type() {
        // Covers _ branch at top-level match
        let result: Result<TokenValues, _> = serde_json::from_str("false");
        assert!(result.is_err());
    }

    // ===== Coverage: number_to_length =====

    #[test]
    fn test_number_to_length_float() {
        // Covers f64 non-integer branch
        let n: serde_json::Number = serde_json::from_str("2.5").unwrap();
        assert_eq!(number_to_length(&n), "10px");
    }

    #[test]
    fn test_number_to_length_float_with_fraction() {
        // Covers f64 branch where result has a fractional part
        let n: serde_json::Number = serde_json::from_str("1.3").unwrap();
        let result = number_to_length(&n);
        assert!(result.ends_with("px"));
        assert!(result.contains("5.2")); // 1.3 * 4 = 5.2
    }

    // ===== Coverage: write_themed_css_vars edge cases =====

    #[test]
    fn test_write_themed_css_vars_empty() {
        // Covers early return for empty themes
        let theme = Theme::default();
        let css = theme.to_css();
        assert_eq!(css, "");
    }

    #[test]
    fn test_token_values_deserialize_invalid_object() {
        // Covers _ branch with Value::Object
        let result: Result<TokenValues, _> = serde_json::from_str(r#"{"a":1}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_length_css_three_variants_sort_order() {
        // Covers all 3 sort_by branches: default first, then alphabetical
        let mut theme = Theme::default();
        theme.add_length("default", "sm", vec![Some("4px".to_string())]);
        theme.add_length("dark", "sm", vec![Some("8px".to_string())]);
        theme.add_length("dim", "sm", vec![Some("6px".to_string())]);

        let css = theme.to_css();
        assert_debug_snapshot!(css);
    }

    #[test]
    fn test_shadow_alias_deserializes_to_shadows() {
        let theme: Theme = serde_json::from_str(
            r#"{
                "shadow": {
                    "light": {
                        "card": ["0 1px 2px #0003", null, "0 4px 8px #0003"]
                    }
                }
            }"#,
        )
        .unwrap();

        let shadow = theme.shadows.get("light").unwrap().get("card").unwrap();
        assert_eq!(
            shadow.0,
            vec![
                Some("0 1px 2px #0003".to_string()),
                None,
                Some("0 4px 8px #0003".to_string())
            ]
        );
    }

    #[test]
    fn test_get_shadow_token_levels() {
        let mut theme = Theme::default();
        theme.add_shadow(
            "default",
            "sm",
            vec![
                Some("0 1px 2px rgba(0,0,0,.1)".to_string()),
                None,
                Some("0 2px 4px rgba(0,0,0,.2)".to_string()),
            ],
        );
        theme.add_shadow(
            "default",
            "md",
            vec![Some("0 4px 8px rgba(0,0,0,.1)".to_string())],
        );

        let levels = theme.get_shadow_token_levels();
        assert_eq!(levels.get("sm").unwrap(), &vec![0u8, 2]);
        assert_eq!(levels.get("md").unwrap(), &vec![0u8]);
    }

    #[test]
    fn test_get_default_shadow_value() {
        let mut theme = Theme::default();
        theme.add_shadow(
            "default",
            "card",
            vec![Some("0 1px 2px #0003".to_string()), None],
        );

        assert_eq!(
            theme.get_default_shadow_value("card"),
            Some("0 1px 2px #0003")
        );
        assert_eq!(theme.get_default_shadow_value("nonexistent"), None);

        // No shadows at all
        let empty = Theme::default();
        assert_eq!(empty.get_default_shadow_value("card"), None);
    }

    // ===== Coverage: push_typography_property edge cases =====

    #[test]
    fn test_push_typography_property_none_value() {
        // Covers early return when value is None
        let mut css = String::new();
        push_typography_property(
            &mut css,
            "font-family",
            None,
            &|out: &mut String, v: &str| out.push_str(v),
        );
        assert_eq!(css, "");
    }

    #[test]
    fn test_push_typography_property_empty_value() {
        // Covers early return when trimmed value is empty
        let mut css = String::new();
        push_typography_property(
            &mut css,
            "font-family",
            Some(""),
            &|out: &mut String, v: &str| out.push_str(v),
        );
        assert_eq!(css, "");

        // Whitespace-only also returns early
        let mut css = String::new();
        push_typography_property(
            &mut css,
            "font-family",
            Some("   "),
            &|out: &mut String, v: &str| out.push_str(v),
        );
        assert_eq!(css, "");
    }

    #[test]
    fn test_push_typography_property_appends_separator() {
        // Empty css → no leading semicolon
        let mut css = String::new();
        push_typography_property(
            &mut css,
            "font-family",
            Some("Arial"),
            &|out: &mut String, v: &str| out.push_str(v),
        );
        assert_eq!(css, "font-family:Arial");

        // Non-empty css → prepends ';' before declaration
        push_typography_property(
            &mut css,
            "font-size",
            Some("16px"),
            &|out: &mut String, v: &str| out.push_str(v),
        );
        assert_eq!(css, "font-family:Arial;font-size:16px");
    }

    // ===== Coverage: push_css_declaration / push_css_variable separators =====

    #[test]
    fn test_push_css_declaration_separator() {
        let mut css = String::new();
        push_css_declaration(&mut css, "color-scheme:light");
        // First call → no separator
        assert_eq!(css, "color-scheme:light");

        // Second call on non-empty buffer → prepends ';'
        push_css_declaration(&mut css, "color:red");
        assert_eq!(css, "color-scheme:light;color:red");
    }

    #[test]
    fn test_push_css_variable_separator() {
        let mut css = String::new();
        push_css_variable(&mut css, "primary", "#000");
        // First call → no separator
        assert_eq!(css, "--primary:#000");

        // Second call → prepends ';'
        push_css_variable(&mut css, "secondary", "#fff");
        assert_eq!(css, "--primary:#000;--secondary:#fff");
    }
}
