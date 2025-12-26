use css::optimize_value::optimize_value;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// ColorEntry stores both the original key (for TypeScript interface) and CSS key (for CSS variables)
#[derive(Debug, Clone, Serialize)]
pub struct ColorEntry {
    /// Original key with dots for TypeScript interface (e.g., "gray.100")
    pub interface_key: String,
    /// CSS variable key with dashes (e.g., "gray-100")
    pub css_key: String,
    /// Color value
    pub value: String,
}

/// ColorTheme stores flattened color entries
/// Supports:
/// - Simple: `primary: "#000"` -> interface_key: "primary", css_key: "primary"
/// - Dot notation: `"primary.100": "#000"` -> interface_key: "primary.100", css_key: "primary-100"
/// - Nested object: `hello: { 100: "#000" }` -> interface_key: "hello.100", css_key: "hello-100"
/// - Deep nested: `gray: { light: { 100: "#000" } }` -> interface_key: "gray.light.100", css_key: "gray-light-100"
#[derive(Default, Serialize, Debug)]
pub struct ColorTheme {
    /// Map from css_key to ColorEntry for quick lookup
    entries: HashMap<String, ColorEntry>,
}

/// Recursively flatten a JSON value into ColorEntry list
/// interface_prefix uses dots, css_prefix uses dashes
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
                    format!("{}.{}", interface_prefix, key)
                };
                let new_css_prefix = if css_prefix.is_empty() {
                    key.replace('.', "-")
                } else {
                    format!("{}-{}", css_prefix, key.replace('.', "-"))
                };
                flatten_color_value(&new_interface_prefix, &new_css_prefix, val, result)?;
            }
            Ok(())
        }
        _ => Err(format!(
            "color value for key '{}' must be a string or an object, got {:?}",
            interface_prefix, value
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
            let css_key = key.replace('.', "-");
            flatten_color_value(&key, &css_key, &value, &mut entries).map_err(D::Error::custom)?;
        }

        Ok(ColorTheme { entries })
    }
}

impl ColorTheme {
    pub fn add_color(&mut self, name: &str, value: &str) {
        let css_key = name.replace('.', "-");
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

    /// Get all CSS keys (for CSS variable generation, with dashes)
    pub fn css_keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Get iterator over (css_key, value) pairs for CSS generation
    pub fn css_entries(&self) -> impl Iterator<Item = (&String, &String)> {
        self.entries.iter().map(|(k, e)| (k, &e.value))
    }

    /// Get value by CSS key
    pub fn get(&self, css_key: &str) -> Option<&String> {
        self.entries.get(css_key).map(|e| &e.value)
    }

    /// Check if CSS key exists
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
    pub fn new(
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
                    _ => return Err(format!("Invalid typography property value: {:?}", item)),
                }
            }
            Ok(result)
        }
        _ => Err(format!("Invalid typography property value: {:?}", value)),
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
                // Check if this looks like a traditional typography array (array of objects/nulls)
                // vs a compact format that accidentally starts with an array
                let is_traditional = arr.iter().all(|v| v.is_object() || v.is_null());
                if is_traditional {
                    let mut result = Vec::with_capacity(arr.len());
                    for item in arr {
                        match item {
                            Value::Null => result.push(None),
                            Value::Object(_) => {
                                let typo: Typography = serde_json::from_value(item.clone())
                                    .map_err(D::Error::custom)?;
                                result.push(Some(typo));
                            }
                            _ => {
                                return Err(D::Error::custom(format!(
                                    "Typography array must contain objects or null, got: {:?}",
                                    item
                                )));
                            }
                        }
                    }
                    Ok(Self(result))
                } else {
                    // Top-level array that's not a traditional format is an error
                    Err(D::Error::custom(
                        "Typography value cannot start with an array. Use object format with property-level arrays instead.",
                    ))
                }
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

                // Find the maximum length among all properties
                let max_len = [
                    font_family.len(),
                    font_size.len(),
                    font_weight.len(),
                    line_height.len(),
                    letter_spacing.len(),
                    font_style.len(),
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
                    let fst = font_style.get(i).cloned().unwrap_or(None);

                    // If all properties are None for this level, push None
                    if ff.is_none()
                        && fs.is_none()
                        && fw.is_none()
                        && lh.is_none()
                        && ls.is_none()
                        && fst.is_none()
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
                "Typography must be an object or array, got: {:?}",
                value
            ))),
        }
    }
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

    pub fn get_default_theme(&self) -> Option<String> {
        self.colors
            .keys()
            .find(|k| *k == "default")
            .or_else(|| {
                self.colors
                    .keys()
                    .find(|k| *k == "light")
                    .or_else(|| self.colors.keys().next())
            })
            .cloned()
    }

    pub fn to_css(&self) -> String {
        let mut theme_declaration = String::new();

        let default_theme_key = self.get_default_theme();
        if let Some(default_theme_key) = default_theme_key {
            let entries = {
                let mut col: Vec<_> = self.colors.iter().collect();
                col.sort_by(|a, b| {
                    if *a.0 == default_theme_key {
                        std::cmp::Ordering::Less
                    } else if *b.0 == default_theme_key {
                        std::cmp::Ordering::Greater
                    } else {
                        a.0.cmp(b.0)
                    }
                });
                col
            };
            let single_theme = entries.len() <= 1;
            // if other theme is exists, should use light-dark function
            let other_theme_key = if entries.len() == 2 {
                entries
                    .iter()
                    .find(|(k, _)| *k != &default_theme_key)
                    .map(|(k, _)| k.to_string())
            } else {
                None
            };
            for (theme_name, theme_properties) in entries {
                let mut css_contents = vec![];
                let mut css_color_contents = vec![];
                let theme_key = if *theme_name == *default_theme_key {
                    None
                } else {
                    Some(theme_name)
                };
                if let Some(theme_key) = theme_key {
                    theme_declaration.push_str(format!(":root[data-theme={theme_key}]{{").as_str());
                    css_contents.push("color-scheme:dark".to_string());
                } else {
                    theme_declaration.push_str(":root{".to_string().as_str());
                    if !single_theme {
                        css_contents.push("color-scheme:light".to_string());
                    }
                }
                for (prop, value) in theme_properties.css_entries() {
                    let optimized_value = optimize_value(value);
                    if theme_key.is_some() {
                        if other_theme_key.is_none()
                            && let Some(default_value) =
                                self.colors.get(&default_theme_key).and_then(|v| {
                                    v.get(prop).and_then(|v| {
                                        if optimize_value(v) == optimized_value {
                                            None
                                        } else {
                                            Some(optimized_value)
                                        }
                                    })
                                })
                        {
                            css_color_contents.push(format!("--{prop}:{default_value}"));
                        }
                    } else {
                        let other_theme_value =
                            other_theme_key.as_ref().and_then(|other_theme_key| {
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
                        css_color_contents.push(format!(
                            "--{prop}:{}",
                            if let Some(other_theme_value) = other_theme_value {
                                format!("light-dark({optimized_value},{other_theme_value})")
                            } else {
                                optimized_value
                            }
                        ));
                    }
                }
                theme_declaration.push_str(
                    [css_contents, css_color_contents]
                        .concat()
                        .join(";")
                        .as_str(),
                );
                theme_declaration.push('}');
            }
        }
        let mut css = theme_declaration;
        let mut level_map = BTreeMap::<u8, Vec<String>>::new();
        for ty in self.typography.iter() {
            for (idx, t) in ty.1.0.iter().enumerate() {
                if let Some(t) = t {
                    let css_content = [
                        t.font_family
                            .as_ref()
                            .map(|v| format!("font-family:{}", optimize_value(v)))
                            .unwrap_or("".to_string()),
                        t.font_size
                            .as_ref()
                            .map(|v| format!("font-size:{}", optimize_value(v)))
                            .unwrap_or("".to_string()),
                        t.font_weight
                            .as_ref()
                            .map(|v| format!("font-weight:{}", optimize_value(v)))
                            .unwrap_or("".to_string()),
                        t.line_height
                            .as_ref()
                            .map(|v| format!("line-height:{}", optimize_value(v)))
                            .unwrap_or("".to_string()),
                        t.letter_spacing
                            .as_ref()
                            .map(|v| format!("letter-spacing:{}", optimize_value(v)))
                            .unwrap_or("".to_string()),
                    ]
                    .iter()
                    .filter_map(|v| {
                        let v = v.trim();
                        if v.is_empty() { None } else { Some(v) }
                    })
                    .collect::<Vec<&str>>()
                    .join(";");

                    if !css_content.is_empty() {
                        level_map
                            .entry(idx as u8)
                            .or_default()
                            .push(format!(".typo-{}{{{}}}", ty.0, css_content));
                    }
                }
            }
        }
        for (level, css_vec) in level_map {
            if level == 0 {
                css.push_str(css_vec.join("").as_str());
            } else if let Some(media) = self
                .breakpoints
                .get(level as usize)
                .map(|v| format!("(min-width:{v}px)"))
            {
                css.push_str(format!("@media{media}{{{}}}", css_vec.join("")).as_str());
            }
        }
        css
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;

    #[test]
    fn to_css_from_theme() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");

        assert_eq!(color_theme.css_keys().count(), 1);

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

        // Helper to create a ColorTheme with a single color
        fn make_color_theme(name: &str, value: &str) -> ColorTheme {
            let mut ct = ColorTheme::default();
            ct.add_color(name, value);
            ct
        }

        let mut theme = Theme::default();
        theme.add_color_theme("default", make_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("a", make_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_color_theme("primary", "#000"));
        theme.add_color_theme("a", make_color_theme("primary", "#000"));
        theme.add_color_theme("c", make_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_color_theme("primary", "#001"));
        theme.add_color_theme("a", make_color_theme("primary", "#002"));
        theme.add_color_theme("c", make_color_theme("primary", "#000"));
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
            r##"{
                "colors": {
                    "light": {
                        "gray": {
                            "100": 123
                        }
                    }
                }
            }"##,
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
        let css_keys: Vec<_> = light.css_keys().cloned().collect();

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

        let interface_keys: Vec<_> = light.interface_keys().cloned().collect();
        let css_keys: Vec<_> = light.css_keys().cloned().collect();

        // Interface key uses dots
        assert!(interface_keys.contains(&"a.b.c".to_string()));
        // CSS key uses dashes
        assert!(css_keys.contains(&"a-b-c".to_string()));
    }

    #[test]
    fn test_compact_typography_format() {
        // Test new compact format with property-level arrays
        let theme: Theme = serde_json::from_str(
            r##"{
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
            }"##,
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
            r##"{
                "typography": {
                    "body": {
                        "fontFamily": "Pretendard",
                        "fontSize": ["14px", null, "16px"],
                        "fontWeight": [500, null, 600],
                        "lineHeight": [1.3, null, 1.5]
                    }
                }
            }"##,
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
            r##"{
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
            }"##,
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
            r##"{
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
            }"##,
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
            r##"{
                "typography": {
                    "h1": {
                        "fontFamily": "Pretendard",
                        "fontSize": ["38px", null, null, null, "52px"],
                        "fontWeight": 800,
                        "lineHeight": 1.3
                    }
                }
            }"##,
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
            r##"{
                "typography": {
                    "h1": ["38px", null, "52px"]
                }
            }"##,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with an array"));
    }

    #[test]
    fn test_mixed_typography_formats() {
        // Test that both formats can coexist in the same theme
        let theme: Theme = serde_json::from_str(
            r##"{
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
            }"##,
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
            r##"{
                "typography": {
                    "h1": {
                        "fontFamily": null,
                        "fontSize": ["14px", null, "16px"]
                    }
                }
            }"##,
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
            r##"{
                "typography": {
                    "h1": {
                        "fontSize": ["14px", {"invalid": "object"}, "16px"]
                    }
                }
            }"##,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_typo_prop_invalid_single_value() {
        // Test that invalid single value fails
        let result: Result<Theme, _> = serde_json::from_str(
            r##"{
                "typography": {
                    "h1": {
                        "fontSize": true
                    }
                }
            }"##,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_typography_invalid_type() {
        // Test that typography with invalid type (string) fails
        let result: Result<Theme, _> = serde_json::from_str(
            r##"{
                "typography": {
                    "h1": "invalid string"
                }
            }"##,
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
            r##"{
                "typography": {
                    "empty": {}
                }
            }"##,
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
            r##"{
                "typography": {
                    "h1": {
                        "letterSpacing": ["-0.02em", null, "-0.03em"]
                    }
                }
            }"##,
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
        assert_eq!(color_theme.css_keys().count(), 0);
        assert_eq!(color_theme.interface_keys().count(), 0);
        assert_eq!(color_theme.css_entries().count(), 0);
        assert!(!color_theme.contains_key("any"));
        assert!(color_theme.get("any").is_none());
    }

    #[test]
    fn test_traditional_typography_with_invalid_item() {
        // Test that traditional array with invalid item (not object/null) fails
        let result: Result<Theme, _> = serde_json::from_str(
            r##"{
                "typography": {
                    "h1": [
                        { "fontFamily": "Arial" },
                        "invalid string item",
                        null
                    ]
                }
            }"##,
        );
        // This should fail because "invalid string item" is not null or object
        // But the current implementation detects this as non-traditional and fails differently
        assert!(result.is_err());
    }

    #[test]
    fn test_compact_typography_different_array_lengths() {
        // Test when different properties have different array lengths
        let theme: Theme = serde_json::from_str(
            r##"{
                "typography": {
                    "h1": {
                        "fontSize": ["14px", "16px"],
                        "fontWeight": ["400", "500", "600", "700"]
                    }
                }
            }"##,
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
            r##"{
                "typography": {
                    "h1": {
                        "lineHeight": [1.2, 1.5, 1.8],
                        "fontWeight": [400.5, 500, 600]
                    }
                }
            }"##,
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
}
