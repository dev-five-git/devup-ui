use css::optimize_value::optimize_value;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct ColorTheme(pub HashMap<String, String>);

impl ColorTheme {
    pub fn add_color(&mut self, name: &str, value: &str) {
        self.0.insert(name.to_string(), value.to_string());
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
pub struct Typographies(Vec<Option<Typography>>);

impl From<Vec<Option<Typography>>> for Typographies {
    fn from(v: Vec<Option<Typography>>) -> Self {
        Self(v)
    }
}
impl<'de> Deserialize<'de> for Typographies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ArrayOrSingle {
            Array(Vec<Option<Typography>>),
            Single(Typography),
        }
        match ArrayOrSingle::deserialize(deserializer)? {
            ArrayOrSingle::Array(v) => Ok(Self(v)),
            ArrayOrSingle::Single(v) => Ok(Self(vec![Some(v)])),
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
                for (prop, value) in theme_properties.0.iter() {
                    let optimized_value = optimize_value(value);
                    if theme_key.is_some() {
                        if other_theme_key.is_none()
                            && let Some(default_value) =
                                self.colors.get(&default_theme_key).and_then(|v| {
                                    v.0.get(prop).and_then(|v| {
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
                                    v.0.get(prop).and_then(|v| {
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
                            .clone()
                            .map(|v| format!("font-family:{v}"))
                            .unwrap_or("".to_string()),
                        t.font_size
                            .clone()
                            .map(|v| format!("font-size:{v}"))
                            .unwrap_or("".to_string()),
                        t.font_weight
                            .clone()
                            .map(|v| format!("font-weight:{v}"))
                            .unwrap_or("".to_string()),
                        t.line_height
                            .clone()
                            .map(|v| format!("line-height:{v}"))
                            .unwrap_or("".to_string()),
                        t.letter_spacing
                            .clone()
                            .map(|v| format!("letter-spacing:{v}"))
                            .unwrap_or("".to_string()),
                    ]
                    .iter()
                    .map(|v| v.trim())
                    .filter(|v| !v.is_empty())
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

        assert_eq!(color_theme.0.keys().count(), 1);

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
        theme.add_color_theme(
            "default",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "dark",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme(
            "light",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "dark",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme(
            "a",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "b",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme(
            "light",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "b",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "a",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "c",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme(
            "light",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme(
            "light",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "b",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#001".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "a",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#002".to_string());
                map
            }),
        );

        theme.add_color_theme(
            "c",
            ColorTheme({
                let mut map = HashMap::new();
                map.insert("primary".to_string(), "#000".to_string());
                map
            }),
        );
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
}
