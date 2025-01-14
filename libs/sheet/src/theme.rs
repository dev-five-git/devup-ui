use std::collections::HashMap;

pub struct ColorTheme {
    data: HashMap<String, String>,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorTheme {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_color(&mut self, name: String, value: String) {
        self.data.insert(name, value);
    }

    pub fn get_color(&self, name: &str) -> Option<&String> {
        self.data.get(name)
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.data.iter()
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.data.keys()
    }
}

pub struct Color {
    pub themes: HashMap<String, ColorTheme>,
}

impl Default for Color {
    fn default() -> Self {
        Self::new()
    }
}

impl Color {
    pub fn new() -> Self {
        Self {
            themes: HashMap::new(),
        }
    }

    pub fn add_theme(&mut self, name: String, theme: ColorTheme) {
        self.themes.insert(name, theme);
    }

    pub fn get_theme(&self, name: &str) -> Option<&ColorTheme> {
        self.themes.get(name)
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
            for (theme_name, theme_properties) in self.themes.iter() {
                let theme_key = if *theme_name == *default_theme_key {
                    None
                } else {
                    Some(theme_name)
                };
                if theme_properties.is_empty() {
                    continue;
                }
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
    pub font_family: String,
    pub font_size: String,
    pub font_weight: String,
    pub line_height: String,
    pub letter_spacing: String,
    pub level: u8,
}

impl Typography {
    pub fn new(
        font_family: String,
        font_size: String,
        font_weight: String,
        line_height: String,
        letter_spacing: String,
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
    pub typography: HashMap<String, Vec<Typography>>,
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

impl Theme {
    pub fn new() -> Self {
        Self {
            colors: Color::new(),
            break_points: vec![0, 480, 768, 992, 1280],
            typography: HashMap::new(),
        }
    }

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

    pub fn add_color_theme(&mut self, name: String, theme: ColorTheme) {
        self.colors.add_theme(name, theme);
    }

    pub fn add_typography(&mut self, name: String, typography: Vec<Typography>) {
        self.typography.insert(name, typography);
    }

    pub fn to_css(&self) -> String {
        let mut css = self.colors.to_css();
        for ty in self.typography.iter() {
            for t in ty.1.iter() {
                if t.level == 0 {
                    css.push_str(
                        format!(
                            ".typo-{}{{font-family:{};font-size:{};font-weight:{};line-height:{};letter-spacing:{}}}",
                            ty.0, t.font_family, t.font_size, t.font_weight, t.line_height, t.letter_spacing
                        )
                            .as_str(),
                    );
                } else {
                    let media = self
                        .break_points
                        .get(t.level as usize)
                        .map(|v| format!("(min-width:{}px)", v));
                    if let Some(media) = media {
                        css.push_str(
                            format!(
                                "\n@media {}{{.typo-{}{{font-family:{};font-size:{};font-weight:{};line-height:{};letter-spacing:{}}}}}",
                                media, ty.0, t.font_family, t.font_size, t.font_weight, t.line_height, t.letter_spacing
                            )
                                .as_str(),
                        );
                    }
                }
            }
        }
        css
    }

    pub fn get_color_theme(&self, name: &str) -> Option<&ColorTheme> {
        self.colors.get_theme(name)
    }
}
