use css::optimize_value::optimize_value;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    /// Map from `css_key` to `ColorEntry`, ordered so the CSS is the same on every run
    entries: BTreeMap<String, ColorEntry>,
}

/// Whether `name` can name a theme token: its CSS variable, dots turned into
/// dashes, is an identifier the `$token` syntax of style values reaches
fn is_token_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn token_name_error(kind: &str, name: &str) -> String {
    format!(
        "{kind} token '{name}' is not a valid token name: start with a letter, a digit or '_', then use letters, digits, '-', '_' and '.'"
    )
}

/// The error for two tokens ending up as one CSS variable, names sorted so
/// the message does not depend on which is read first
fn token_collision_error(kind: &str, first: &str, second: &str, css_key: &str) -> String {
    let (first, second) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    format!(
        "{kind} tokens '{first}' and '{second}' both become the CSS variable --{css_key}: rename one"
    )
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
    result: &mut BTreeMap<String, ColorEntry>,
) -> Result<(), String> {
    match value {
        Value::String(s) => {
            if let Some(existing) = result.get(css_prefix) {
                return Err(token_collision_error(
                    "color",
                    &existing.interface_key,
                    interface_prefix,
                    css_prefix,
                ));
            }
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
                if !is_token_name(key) {
                    return Err(token_name_error("color", key));
                }
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

        let raw: BTreeMap<String, Value> = BTreeMap::deserialize(deserializer)?;
        let mut entries = BTreeMap::new();

        for (key, value) in raw {
            if !is_token_name(&key) {
                return Err(D::Error::custom(token_name_error("color", &key)));
            }
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
    #[serde(default)]
    pub font_style: Option<String>,
    #[serde(default)]
    pub text_transform: Option<String>,
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
            font_style: None,
            text_transform: None,
        }
    }

    /// The CSS declarations of this preset, in output order
    fn properties(&self) -> [(&'static str, Option<&str>); 7] {
        [
            ("font-family", self.font_family.as_deref()),
            ("font-size", self.font_size.as_deref()),
            ("font-style", self.font_style.as_deref()),
            ("font-weight", self.font_weight.as_deref()),
            ("line-height", self.line_height.as_deref()),
            ("letter-spacing", self.letter_spacing.as_deref()),
            ("text-transform", self.text_transform.as_deref()),
        ]
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

                let font_style = obj
                    .get("fontStyle")
                    .map(deserialize_typo_prop)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_else(|| vec![None]);

                let text_transform = obj
                    .get("textTransform")
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
                    font_style.len(),
                    text_transform.len(),
                ]
                .into_iter()
                .max()
                .unwrap_or(1);

                // Build typography for each breakpoint level
                let mut result = Vec::with_capacity(max_len);
                for i in 0..max_len {
                    let typography = Typography {
                        font_family: font_family.get(i).cloned().flatten(),
                        font_size: font_size.get(i).cloned().flatten(),
                        font_weight: font_weight.get(i).cloned().flatten(),
                        line_height: line_height.get(i).cloned().flatten(),
                        letter_spacing: letter_spacing.get(i).cloned().flatten(),
                        font_style: font_style.get(i).cloned().flatten(),
                        text_transform: text_transform.get(i).cloned().flatten(),
                    };
                    // A level setting no property is a gap in the responsive list
                    result.push(
                        typography
                            .properties()
                            .iter()
                            .any(|(_, value)| value.is_some())
                            .then_some(typography),
                    );
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
                        // This branch is cold for normal themes, but the public
                        // breakpoint and token APIs permit larger responsive sets.
                        // Keep the fallback functional in debug builds as well as
                        // release builds and assert the invariant established by the
                        // branch above.
                        debug_assert!(
                            level >= 16,
                            "overflow branch requires a level outside the u16 mask"
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
    check_token_names("length", "length", &raw).map_err(serde::de::Error::custom)?;
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

/// The native color scheme a theme variant renders with: form controls,
/// scrollbars and the branch `light-dark()` picks
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Light,
    Dark,
}

impl ColorScheme {
    const fn declaration(self) -> &'static str {
        match self {
            Self::Light => "color-scheme:light",
            Self::Dark => "color-scheme:dark",
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    #[serde(default, deserialize_with = "deserialize_color_themes")]
    pub colors: BTreeMap<String, ColorTheme>,
    /// The color scheme of each variant named here; `dark` is dark and any
    /// other variant light unless listed
    #[serde(default)]
    pub color_scheme: BTreeMap<String, ColorScheme>,
    #[serde(default = "default_breakpoints")]
    pub breakpoints: Vec<u16>,
    #[serde(default)]
    pub typography: BTreeMap<String, Typographies>,
    #[serde(default, deserialize_with = "deserialize_length_themes")]
    pub length: BTreeMap<String, LengthTheme>,
    #[serde(
        default,
        alias = "shadow",
        deserialize_with = "deserialize_shadow_themes"
    )]
    pub shadows: BTreeMap<String, ShadowTheme>,
}

/// Deserialize the color variants, naming the variant a token error is in
fn deserialize_color_themes<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, ColorTheme>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: BTreeMap<String, Value> = BTreeMap::deserialize(deserializer)?;
    raw.into_iter()
        .map(|(variant, value)| {
            ColorTheme::deserialize(value)
                .map(|theme| (variant.clone(), theme))
                .map_err(|error| {
                    serde::de::Error::custom(format!("theme.colors.{variant}: {error}"))
                })
        })
        .collect()
}

/// Check the names of the tokens of each variant of `themes`: each must be a
/// token name, and no two may become the same CSS variable
fn check_token_names<T>(
    kind: &str,
    field: &str,
    themes: &BTreeMap<String, BTreeMap<String, T>>,
) -> Result<(), String> {
    for (variant, tokens) in themes {
        let mut css_keys = BTreeMap::new();
        for name in tokens.keys() {
            if !is_token_name(name) {
                return Err(format!(
                    "theme.{field}.{variant}: {}",
                    token_name_error(kind, name)
                ));
            }
            let css_key = css_key_from(name).into_owned();
            if let Some(existing) = css_keys.insert(css_key.clone(), name) {
                return Err(format!(
                    "theme.{field}.{variant}: {}",
                    token_collision_error(kind, existing, name, &css_key)
                ));
            }
        }
    }
    Ok(())
}

fn deserialize_shadow_themes<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<String, ShadowTheme>, D::Error>
where
    D: Deserializer<'de>,
{
    let themes: BTreeMap<String, ShadowTheme> = BTreeMap::deserialize(deserializer)?;
    check_token_names("shadow", "shadow", &themes).map_err(serde::de::Error::custom)?;
    Ok(themes)
}

fn default_breakpoints() -> Vec<u16> {
    vec![0, 480, 768, 992, 1280, 1600]
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: Default::default(),
            color_scheme: BTreeMap::new(),
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

    fn color_scheme_of(&self, variant: &str) -> ColorScheme {
        self.color_scheme
            .get(variant)
            .copied()
            .unwrap_or(if variant == "dark" {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            })
    }

    /// Declarations of typography preset `name` applied from breakpoint `level`
    /// up: preset entries at or below `level` merge into `level`, wider ones
    /// keep their own breakpoint. Values resolve like the `.typo-*` classes.
    #[must_use]
    pub fn typography_declarations(
        &self,
        name: &str,
        level: u8,
    ) -> Vec<(u8, &'static str, String)> {
        let mut declarations: Vec<(u8, &'static str, String)> = vec![];
        let Some(typography) = self.typography.get(name) else {
            return declarations;
        };
        for (index, entry) in typography.0.iter().enumerate() {
            let Some(entry) = entry else {
                continue;
            };
            let target = level.max(index as u8);
            for (property, value) in entry.properties() {
                let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
                    continue;
                };
                let resolved = value.strip_prefix('$').map_or_else(
                    || optimize_value(value).into_owned(),
                    |token| format!("var(--{token})"),
                );
                if let Some(existing) = declarations
                    .iter_mut()
                    .find(|(l, p, _)| *l == target && *p == property)
                {
                    existing.2 = resolved;
                } else {
                    declarations.push((target, property, resolved));
                }
            }
        }
        declarations
    }

    /// The CSS names of the colors every theme defines
    #[must_use]
    pub fn get_color_token_names(&self) -> BTreeSet<String> {
        self.colors
            .values()
            .flat_map(|theme| theme.css_entries().map(|(name, _)| name.clone()))
            .collect()
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

        if let Some(default_key) = default_variant_key(&self.colors)
            && let Some(default_colors) = self.colors.get(default_key)
        {
            let single_theme = self.colors.len() <= 1;
            let mut variants: Vec<_> = self.colors.iter().collect();
            sort_variants_default_first(&mut variants, default_key);
            let default_values: BTreeMap<&str, Cow<str>> = default_colors
                .css_entries()
                .map(|(key, value)| (key.as_str(), optimize_value(value)))
                .collect();
            // `light-dark()` holds one light and one dark value, so it encodes a
            // light default paired with a single dark variant
            let dark_partner = match variants.as_slice() {
                [_, (name, _)]
                    if self.color_scheme_of(default_key) == ColorScheme::Light
                        && self.color_scheme_of(name) == ColorScheme::Dark =>
                {
                    Some(name.as_str())
                }
                _ => None,
            };
            let partner_colors = dark_partner.and_then(|partner| self.colors.get(partner));
            for (name, colors) in variants {
                let is_default = name.as_str() == default_key;
                let mut contents = String::new();
                if !single_theme {
                    push_css_declaration(&mut contents, self.color_scheme_of(name).declaration());
                }
                for (key, value) in colors.css_entries() {
                    let default_value = default_values.get(key.as_str());
                    let value = optimize_value(value);
                    if is_default {
                        let dark = partner_colors
                            .and_then(|partner| partner.get(key))
                            .map(|dark| optimize_value(dark))
                            .filter(|dark| *dark != value);
                        match dark {
                            Some(dark) => push_css_variable(
                                &mut contents,
                                key,
                                &format!("light-dark({value},{dark})"),
                            ),
                            None => push_css_variable(&mut contents, key, &value),
                        }
                    } else {
                        // The dark partner's shared colors are in the default's `light-dark()`
                        let shown = match default_value {
                            None => true,
                            Some(_) if Some(name.as_str()) == dark_partner => false,
                            Some(default_value) => *default_value != value,
                        };
                        if shown {
                            push_css_variable(&mut contents, key, &value);
                        }
                    }
                }
                push_theme_root(
                    &mut theme_declaration,
                    (!is_default).then_some(name.as_str()),
                );
                theme_declaration.push('{');
                theme_declaration.push_str(&contents);
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
                    for (property, value) in t.properties() {
                        push_typography_property(&mut css_content, property, value, &resolve_into);
                    }

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
                                    .map(|d| ((name.as_str(), idx), optimize_value(d).into_owned()))
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        };

        for (variant_name, token_theme) in &sorted_variants {
            let variant = (*variant_name != default_key).then_some(variant_name.as_str());
            let write_selector = |css: &mut String| push_theme_root(css, variant);

            // Group variables by breakpoint level without allocating one String per variable.
            let mut level_map = BTreeMap::<usize, String>::new();
            for (name, values) in *token_theme {
                // `name` is invariant across the `idx` iteration, so borrow it as `&str`
                // once here instead of re-`as_str()`ing it inside every value probe/push.
                let name_str = name.as_str();
                for (idx, val) in values.0.iter().enumerate() {
                    if let Some(v) = val {
                        let optimized = optimize_value(v);
                        let is_same_as_default = variant.is_some()
                            && default_optimized
                                .get(&(name_str, idx))
                                .is_some_and(|d| *d == optimized);
                        if !is_same_as_default {
                            let vars = level_map.entry(idx).or_default();
                            if !vars.is_empty() {
                                vars.push(';');
                            }
                            vars.push_str("--");
                            vars.push_str(&css_key_from(name_str));
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

/// Write the `:root` selector of theme `variant`, of the default theme for
/// `None`; a name that is not a CSS identifier is quoted
fn push_theme_root(css: &mut String, variant: Option<&str>) {
    css.push_str(":root");
    let Some(variant) = variant else {
        return;
    };
    css.push_str("[data-theme=");
    let mut chars = variant.chars();
    let identifier = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if identifier {
        css.push_str(variant);
    } else {
        css.push('"');
        for c in variant.chars() {
            if matches!(c, '"' | '\\') {
                css.push('\\');
            }
            css.push(c);
        }
        css.push('"');
    }
    css.push(']');
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
    fn test_color_token_names_cover_every_theme() {
        let theme: Theme = serde_json::from_str(
            r##"{"colors": {"light": {"primary": "#100"}, "dark": {"primary": "#200", "text.muted": "#300"}}}"##,
        )
        .unwrap();

        assert_eq!(
            theme.get_color_token_names(),
            BTreeSet::from([String::from("primary"), String::from("text-muted")])
        );
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

    fn theme_css(json: &str) -> String {
        serde_json::from_str::<Theme>(json).unwrap().to_css()
    }

    fn theme_error(json: &str) -> String {
        serde_json::from_str::<Theme>(json).unwrap_err().to_string()
    }

    #[test]
    fn test_color_schemes_and_variant_only_tokens() {
        // `light-dark()` only encodes a light default paired with a dark variant,
        // and a token one variant alone defines is still defined there
        assert_eq!(
            theme_css(
                r##"{"colors":{"default":{"a":"#111","same":"#222"},"dark":{"a":"#eee","same":"#222","darkOnly":"#123"}}}"##
            ),
            ":root{color-scheme:light;--a:light-dark(#111,#EEE);--same:#222}:root[data-theme=dark]{color-scheme:dark;--darkOnly:#123}"
        );
        assert_eq!(
            theme_css(
                r##"{"colors":{"default":{"a":"#111"},"blue":{"a":"#00f","blueOnly":"#0f0"}}}"##
            ),
            ":root{color-scheme:light;--a:#111}:root[data-theme=blue]{color-scheme:light;--a:#00F;--blueOnly:#0F0}"
        );
        assert_eq!(
            theme_css(
                r##"{"colors":{"default":{"a":"#111"},"midnight":{"a":"#000"}},"colorScheme":{"midnight":"dark"}}"##
            ),
            ":root{color-scheme:light;--a:light-dark(#111,#000)}:root[data-theme=midnight]{color-scheme:dark}"
        );
        assert_eq!(
            theme_css(
                r##"{"colors":{"default":{"a":"#111"},"dark":{"a":"#eee"},"solarized":{"a":"#abc","own":"#fed"}}}"##
            ),
            ":root{color-scheme:light;--a:#111}:root[data-theme=dark]{color-scheme:dark;--a:#EEE}:root[data-theme=solarized]{color-scheme:light;--a:#ABC;--own:#FED}"
        );
        assert_eq!(
            theme_css(
                r##"{"colors":{"default":{"a":"#111"},"high contrast":{"a":"#000"},"q\"b\\":{"a":"#fff"}}}"##
            ),
            r#":root{color-scheme:light;--a:#111}:root[data-theme="high contrast"]{color-scheme:light;--a:#000}:root[data-theme="q\"b\\"]{color-scheme:light;--a:#FFF}"#
        );
    }

    #[test]
    fn test_token_names_are_checked() {
        assert_eq!(
            theme_error(r##"{"colors":{"default":{"a-b":"#111","a":{"b":"#222"}}}}"##),
            "theme.colors.default: color tokens 'a-b' and 'a.b' both become the CSS variable --a-b: rename one at line 1 column 54"
        );
        for (json, error) in [
            (
                r##"{"colors":{"default":{"${bad}":"#111"}}}"##,
                "theme.colors.default: color token '${bad}' is not a valid token name",
            ),
            (
                r##"{"colors":{"default":{"a":{"b c":"#111"}}}}"##,
                "theme.colors.default: color token 'b c' is not a valid token name",
            ),
            (
                r#"{"length":{"default":{"-x":"4px"}}}"#,
                "theme.length.default: length token '-x' is not a valid token name",
            ),
            (
                r#"{"length":{"default":{"a.b":"4px","a-b":"8px"}}}"#,
                "theme.length.default: length tokens 'a-b' and 'a.b' both become the CSS variable --a-b",
            ),
            (
                r#"{"shadow":{"default":{"a b":"0 1px #000"}}}"#,
                "theme.shadow.default: shadow token 'a b' is not a valid token name",
            ),
        ] {
            assert!(
                theme_error(json).starts_with(error),
                "{json}: {}",
                theme_error(json)
            );
        }
    }

    #[test]
    fn test_dotted_length_and_shadow_names_become_dashed_variables() {
        assert_eq!(
            theme_css(
                r#"{"length":{"default":{"gap.sm":"4px"}},"shadow":{"default":{"card.lg":"0 1px #000"}}}"#
            ),
            ":root{--gap-sm:4px}:root{--card-lg:0 1px #000}"
        );
    }

    #[test]
    fn test_typography_style_and_transform() {
        let css = theme_css(
            r#"{"typography":{"caps":{"fontStyle":["italic",null,"normal"],"textTransform":"uppercase"},"quote":[{"fontStyle":"italic","textTransform":"none"}]}}"#,
        );
        assert_eq!(
            css,
            ".typo-caps{font-style:italic;text-transform:uppercase}.typo-quote{font-style:italic;text-transform:none}@media(min-width:768px){.typo-caps{font-style:normal}}"
        );
        let theme: Theme = serde_json::from_str(
            r#"{"typography":{"caps":{"fontStyle":"italic","textTransform":"uppercase"}}}"#,
        )
        .unwrap();
        assert_eq!(
            theme.typography_declarations("caps", 0),
            vec![
                (0, "font-style", "italic".to_string()),
                (0, "text-transform", "uppercase".to_string()),
            ]
        );
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
    fn test_get_length_token_levels_with_more_than_sixteen_breakpoints() {
        let mut theme = Theme::default();
        theme.update_breakpoints((0u16..18).map(|level| level * 100).collect());

        let mut dark_values = vec![None; 18];
        dark_values[16] = Some("16px".to_string());
        dark_values[17] = Some("17px".to_string());
        theme.add_length("dark", "wide", dark_values);

        let mut default_values = vec![None; 18];
        default_values[2] = Some("2px".to_string());
        default_values[17] = Some("17px".to_string());
        theme.add_length("default", "wide", default_values);

        assert_eq!(theme.get_length_token_levels()["wide"], vec![2, 16, 17]);
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
