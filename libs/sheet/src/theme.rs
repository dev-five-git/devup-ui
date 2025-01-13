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

pub struct Theme {
    pub colors: Color,
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
        }
    }

    pub fn add_color_theme(&mut self, name: String, theme: ColorTheme) {
        self.colors.add_theme(name, theme);
    }

    pub fn get_color_theme(&self, name: &str) -> Option<&ColorTheme> {
        self.colors.get_theme(name)
    }
}
