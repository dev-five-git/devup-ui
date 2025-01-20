use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct ColorTheme {
    data: HashMap<String, String>,
}

impl ColorTheme {
    pub fn add_color(&mut self, name: &str, value: &str) {
        self.data.insert(name.to_string(), value.to_string());
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.data.iter()
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }
}

#[derive(Default)]
pub struct Color {
    pub themes: HashMap<String, ColorTheme>,
}

impl Color {
    pub fn add_theme(&mut self, name: &str, theme: ColorTheme) {
        self.themes.insert(name.to_string(), theme);
    }

    pub fn to_css(&self) -> String {
        let mut theme_declaration = String::new();
        let default_theme_key = self
            .themes
            .keys()
            .find(|k| *k == "default")
            .map(Some)
            .unwrap_or_else(|| self.themes.keys().next());
        if let Some(default_theme_key) = default_theme_key {
            let mut entries: Vec<_> = self.themes.iter().collect();
            entries.sort_by_key(|(k, _)| *k);
            entries.reverse();
            for (theme_name, theme_properties) in entries {
                let theme_key = if *theme_name == *default_theme_key {
                    None
                } else {
                    Some(theme_name)
                };
                if let Some(theme_key) = theme_key {
                    theme_declaration
                        .push_str(format!(":root[data-theme={}]{{", theme_key).as_str());
                } else {
                    theme_declaration.push_str(":root{");
                }
                for (prop, value) in theme_properties.iter() {
                    theme_declaration.push_str(format!("--{}:{};", prop, value).as_str());
                }
                theme_declaration.push_str("}\n");
            }
        }
        theme_declaration
    }
}
pub struct Typography {
    pub font_family: Option<String>,
    pub font_size: Option<String>,
    pub font_weight: Option<String>,
    pub line_height: Option<String>,
    pub letter_spacing: Option<String>,
    pub level: u8,
}

impl Typography {
    pub fn new(
        font_family: Option<String>,
        font_size: Option<String>,
        font_weight: Option<String>,
        line_height: Option<String>,
        letter_spacing: Option<String>,
        level: u8,
    ) -> Self {
        Self {
            font_family,
            font_size,
            font_weight,
            line_height,
            letter_spacing,
            level,
        }
    }
}

pub struct Theme {
    pub colors: Color,
    pub break_points: Vec<u16>,
    pub typography: BTreeMap<String, Vec<Typography>>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            colors: Color::default(),
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
        self.colors.add_theme(name, theme);
    }

    pub fn add_typography(&mut self, name: &str, typography: Vec<Typography>) {
        self.typography.insert(name.to_string(), typography);
    }

    pub fn to_css(&self) -> String {
        let mut css = self.colors.to_css();
        let mut level_map = BTreeMap::<u8, Vec<String>>::new();
        for ty in self.typography.iter() {
            for t in ty.1.iter() {
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
                    .get_mut(&t.level)
                    .map(|v| v.push(typo_css.clone()))
                    .unwrap_or_else(|| {
                        level_map.insert(t.level, vec![typo_css.clone()]);
                    });
            }
        }
        for (level, css_vec) in level_map {
            if level == 0 {
                css.push_str(css_vec.join("").as_str());
            } else {
                let media = self
                    .break_points
                    .get(level as usize)
                    .map(|v| format!("(min-width:{}px)", v));
                if let Some(media) = media {
                    css.push_str(format!("\n@media {}{{{}}}", media, css_vec.join("")).as_str());
                }
            }
        }
        css
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_css_from_theme() {
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");

        assert_eq!(color_theme.keys().count(), 1);

        theme.add_color_theme("default", color_theme);
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#fff");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "default",
            vec![
                Typography::new(
                    Some("Arial".to_string()),
                    Some("16px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                    0,
                ),
                Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                    1,
                ),
            ],
        );

        theme.add_typography(
            "default1",
            vec![Typography::new(
                Some("Arial".to_string()),
                Some("24px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
                1,
            )],
        );
        let css = theme.to_css();
        assert_eq!(
            css,
            ":root{--primary:#000;}\n:root[data-theme=dark]{--primary:#fff;}\n.typo-default{font-family:Arial;font-size:16px;font-weight:400;line-height:1.5;letter-spacing:0.5}\n@media (min-width:480px){.typo-default{font-family:Arial;font-size:24px;font-weight:400;line-height:1.5;letter-spacing:0.5}.typo-default1{font-family:Arial;font-size:24px;font-weight:400;line-height:1.5;letter-spacing:0.5}}"
        );

        assert_eq!(Theme::default().to_css(), "");
        let mut theme = Theme::default();
        theme.add_typography(
            "default",
            vec![Typography::new(None, None, None, None, None, 0)],
        );
        assert_eq!(theme.to_css(), "");
    }
}
