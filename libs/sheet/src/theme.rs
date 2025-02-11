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
    #[serde(default = "default_break_points")]
    pub break_points: Vec<u16>,
    #[serde(default)]
    pub typography: BTreeMap<String, Typographies>,
}

fn default_break_points() -> Vec<u16> {
    vec![0, 480, 768, 992, 1280]
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: Default::default(),
            break_points: vec![0, 480, 768, 992, 1280],
            typography: BTreeMap::new(),
        }
    }
}

impl Theme {
    pub fn update_break_points(&mut self, break_points: Vec<u16>) {
        for (idx, value) in break_points.iter().enumerate() {
            let prev = self.break_points.get_mut(idx);
            if let Some(prev) = prev {
                *prev = *value;
            } else {
                self.break_points.push(*value);
            }
        }
    }

    pub fn add_color_theme(&mut self, name: &str, theme: ColorTheme) {
        self.colors.insert(name.to_string(), theme);
    }

    pub fn add_typography(&mut self, name: &str, typography: Vec<Option<Typography>>) {
        self.typography.insert(name.to_string(), typography.into());
    }

    pub fn to_css(&self) -> String {
        let mut theme_declaration = String::new();

        let default_theme_key = self.colors.keys().find(|k| *k == "default").or_else(|| {
            self.colors
                .keys()
                .find(|k| *k == "light")
                .or_else(|| self.colors.keys().next())
        });
        if let Some(default_theme_key) = default_theme_key {
            let entries = {
                let mut col: Vec<_> = self.colors.iter().collect();
                col.sort_by(|a, b| {
                    if a.0 == default_theme_key {
                        std::cmp::Ordering::Less
                    } else if b.0 == default_theme_key {
                        std::cmp::Ordering::Greater
                    } else {
                        a.0.cmp(b.0)
                    }
                });
                col
            };
            let single_theme = entries.len() <= 1;
            for (theme_name, theme_properties) in entries {
                if let Some(theme_key) = if *theme_name == *default_theme_key {
                    None
                } else {
                    Some(theme_name)
                } {
                    theme_declaration.push_str(
                        format!(":root[data-theme={}]{{{}", theme_key, "color-scheme:dark;")
                            .as_str(),
                    );
                } else {
                    theme_declaration.push_str(
                        format!(
                            ":root{{{}",
                            if single_theme {
                                ""
                            } else {
                                "color-scheme:light;"
                            }
                        )
                        .as_str(),
                    );
                }
                for (prop, value) in theme_properties.0.iter() {
                    theme_declaration.push_str(format!("--{}:{};", prop, value).as_str());
                }
                theme_declaration.push_str("}\n");
            }
        }
        let mut css = theme_declaration;
        let mut level_map = BTreeMap::<u8, Vec<String>>::new();
        for ty in self.typography.iter() {
            for (idx, t) in ty.1 .0.iter().enumerate() {
                if let Some(t) = t {
                    let css_content = format!(
                        "{}{}{}{}{}",
                        t.font_family
                            .clone()
                            .map(|v| format!("font-family:{};", v))
                            .unwrap_or("".to_string()),
                        t.font_size
                            .clone()
                            .map(|v| format!("font-size:{};", v))
                            .unwrap_or("".to_string()),
                        t.font_weight
                            .clone()
                            .map(|v| format!("font-weight:{};", v))
                            .unwrap_or("".to_string()),
                        t.line_height
                            .clone()
                            .map(|v| format!("line-height:{};", v))
                            .unwrap_or("".to_string()),
                        t.letter_spacing
                            .clone()
                            .map(|v| format!("letter-spacing:{}", v))
                            .unwrap_or("".to_string())
                    );
                    if css_content.is_empty() {
                        continue;
                    }
                    let typo_css = format!(".typo-{}{{{}}}", ty.0, css_content);
                    level_map
                        .get_mut(&(idx as u8))
                        .map(|v| v.push(typo_css.clone()))
                        .unwrap_or_else(|| {
                            level_map.insert(idx as u8, vec![typo_css]);
                        });
                }
            }
        }
        for (level, css_vec) in level_map {
            if level == 0 {
                css.push_str(css_vec.join("").as_str());
            } else if let Some(media) = self
                .break_points
                .get(level as usize)
                .map(|v| format!("(min-width:{}px)", v))
            {
                css.push_str(format!("\n@media {}{{{}}}", media, css_vec.join("")).as_str());
            }
        }
        css
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

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
        assert_eq!(
            css,
            ":root{color-scheme:light;--primary:#000;}\n:root[data-theme=dark]{color-scheme:dark;--primary:#fff;}\n.typo-default{font-family:Arial;font-size:16px;font-weight:400;line-height:1.5;letter-spacing:0.5}\n@media (min-width:480px){.typo-default{font-family:Arial;font-size:24px;font-weight:400;line-height:1.5;letter-spacing:0.5}.typo-default1{font-family:Arial;font-size:24px;font-weight:400;line-height:1.5;letter-spacing:0.5}}"
        );

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
    }

    #[test]
    fn update_break_points() {
        let mut theme = Theme::default();
        theme.update_break_points(vec![0, 480, 768, 992, 1280]);
        assert_eq!(theme.break_points, vec![0, 480, 768, 992, 1280]);
        theme.update_break_points(vec![0, 480, 768, 992, 1280, 1600]);
        assert_eq!(theme.break_points, vec![0, 480, 768, 992, 1280, 1600]);
        theme.update_break_points(vec![0, 480, 768, 992, 1280, 1600, 1920]);
        assert_eq!(theme.break_points, vec![0, 480, 768, 992, 1280, 1600, 1920]);
    }
}
