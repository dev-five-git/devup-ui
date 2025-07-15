use css::class_map::{get_class_map, set_class_map};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, StyleProperty, extract};
use once_cell::sync::Lazy;
use sheet::StyleSheet;
use std::collections::HashSet;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static GLOBAL_STYLE_SHEET: Lazy<Mutex<StyleSheet>> =
    Lazy::new(|| Mutex::new(StyleSheet::default()));

#[wasm_bindgen]
pub struct Output {
    code: String,
    styles: HashSet<ExtractStyleValue>,
    map: Option<String>,
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &JsValue);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_str(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = time)]
    fn time(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = timeEnd)]
    fn time_end(s: &str);
}

#[wasm_bindgen]
impl Output {
    /// Get the code
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn map(&self) -> Option<String> {
        self.map.clone()
    }

    /// Get the css
    #[wasm_bindgen(getter)]
    pub fn css(&self) -> Option<String> {
        let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
        let mut collected = false;
        for style in self.styles.iter() {
            match style {
                ExtractStyleValue::Static(st) => {
                    let (cls, _) = match style.extract() {
                        Some(StyleProperty::ClassName(cls)) => (cls, None),
                        Some(StyleProperty::Variable {
                            class_name,
                            variable_name,
                            ..
                        }) => (class_name, Some(variable_name)),
                        None => continue,
                    };
                    if sheet.add_property(
                        &cls,
                        st.property(),
                        st.level(),
                        st.value(),
                        st.selector(),
                        st.style_order(),
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    let (cls, variable) = match style.extract() {
                        Some(StyleProperty::ClassName(cls)) => (cls, None),
                        Some(StyleProperty::Variable {
                            class_name,
                            variable_name,
                            ..
                        }) => (class_name, Some(variable_name)),
                        None => continue,
                    };
                    if sheet.add_property(
                        &cls,
                        dy.property(),
                        dy.level(),
                        &format!("var({})", variable.unwrap()),
                        dy.selector(),
                        dy.style_order(),
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Css(cs) => {
                    if sheet.add_css(&cs.file, &cs.css) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Typography(_) => {}
                ExtractStyleValue::Import(st) => {
                    sheet.add_import(&st.file, &st.url);
                }
            }
        }

        if !collected {
            return None;
        }

        Some(sheet.create_css())
    }
}

#[wasm_bindgen(js_name = "setDebug")]
pub fn set_debug(debug: bool) {
    css::debug::set_debug(debug);
}

#[wasm_bindgen(js_name = "isDebug")]
pub fn is_debug() {
    css::debug::is_debug();
}

#[wasm_bindgen(js_name = "importSheet")]
pub fn import_sheet(sheet_object: JsValue) -> Result<(), JsValue> {
    *GLOBAL_STYLE_SHEET.lock().unwrap() = serde_wasm_bindgen::from_value(sheet_object)
        .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;
    Ok(())
}

#[wasm_bindgen(js_name = "exportSheet")]
pub fn export_sheet() -> Result<String, JsValue> {
    serde_json::to_string(&*GLOBAL_STYLE_SHEET.lock().unwrap())
        .map_err(|e| JsValue::from_str(e.to_string().as_str()))
}

#[wasm_bindgen(js_name = "importClassMap")]
pub fn import_class_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_class_map(
        serde_wasm_bindgen::from_value(sheet_object)
            .map_err(|e| JsValue::from_str(e.to_string().as_str()))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportClassMap")]
pub fn export_class_map() -> Result<String, JsValue> {
    serde_json::to_string(&get_class_map()).map_err(|e| JsValue::from_str(e.to_string().as_str()))
}

#[wasm_bindgen(js_name = "codeExtract")]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_file: &str,
) -> Result<Output, JsValue> {
    let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    sheet.rm_global_css(filename);

    match extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
            css_file: Some(css_file.to_string()),
        },
    ) {
        Ok(output) => Ok(Output {
            code: output.code,
            styles: output.styles,
            map: output.map,
        }),
        Err(error) => Err(JsValue::from_str(error.to_string().as_str())),
    }
}

#[wasm_bindgen(js_name = "registerTheme")]
pub fn register_theme(theme_object: JsValue) -> Result<(), JsValue> {
    GLOBAL_STYLE_SHEET.lock().unwrap().set_theme(
        serde_wasm_bindgen::from_value(theme_object)
            .map_err(|e| JsValue::from_str(e.to_string().as_str()))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "getDefaultTheme")]
pub fn get_default_theme() -> Result<Option<String>, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    Ok(sheet.theme.get_default_theme())
}

#[wasm_bindgen(js_name = "getCss")]
pub fn get_css() -> Result<String, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    Ok(sheet.create_css())
}

#[wasm_bindgen(js_name = "getThemeInterface")]
pub fn get_theme_interface(
    package_name: &str,
    color_interface_name: &str,
    typography_interface_name: &str,
    theme_interface_name: &str,
) -> String {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    let mut color_keys = HashSet::new();
    let mut typography_keys = HashSet::new();
    let mut theme_keys = HashSet::new();
    for color_theme in sheet.theme.colors.values() {
        color_theme.0.keys().for_each(|key| {
            color_keys.insert(key.clone());
        });
    }
    sheet.theme.typography.keys().for_each(|key| {
        typography_keys.insert(key.clone());
    });

    sheet.theme.colors.keys().for_each(|key| {
        theme_keys.insert(key.clone());
    });

    if color_keys.is_empty() && typography_keys.is_empty() {
        String::new()
    } else {
        format!(
            "import \"{}\";declare module \"{}\"{{interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}}}",
            package_name,
            package_name,
            color_interface_name,
            color_keys
                .into_iter()
                .map(|key| format!("${key}:null;"))
                .collect::<Vec<String>>()
                .join(""),
            typography_interface_name,
            typography_keys
                .into_iter()
                .map(|key| format!("{key}:null;"))
                .collect::<Vec<String>>()
                .join(""),
            theme_interface_name,
            theme_keys
                .into_iter()
                // key to pascal
                .map(|key| format!("{key}:null;"))
                .collect::<Vec<String>>()
                .join("")
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use serial_test::serial;
    use sheet::theme::{ColorTheme, Theme};

    #[test]
    #[serial]
    fn test_code_extract() {
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        assert_eq!(get_css().unwrap(), "");

        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            let mut theme = Theme::default();
            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#000");
            theme.add_color_theme("dark", color_theme);

            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#FFF");
            theme.add_color_theme("default", color_theme);
            sheet.set_theme(theme);
        }

        assert_eq!(
            get_css().unwrap(),
            ":root{color-scheme:light;--primary:#FFF;}\n:root[data-theme=dark]{color-scheme:dark;--primary:#000;}\n"
        );
    }

    #[test]
    #[serial]
    fn test_get_theme_interface() {
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            ""
        );

        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            let mut theme = Theme::default();
            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#000");
            theme.add_color_theme("dark", color_theme);
            sheet.set_theme(theme);
        }
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{$primary:null;}interface TypographyInterface{}interface ThemeInterface{dark:null;}}"
        );
    }

    #[test]
    fn deserialize_theme() {
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default": [
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": 400,
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    },
                    {
                        "fontFamily": "Arial",
                        "fontSize": "24px",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
                ]
            }
        }"##,
            )
            .unwrap();
            assert_eq!(theme.breakpoints, vec![0, 480, 768, 992, 1280]);
            assert_debug_snapshot!(theme.to_css());
        }
        {
            let theme: Theme = serde_json::from_str(
                r##"{
            "colors": {
                "default": {
                    "primary": "#000"
                },
                "dark": {
                    "primary": "#fff"
                }
            },
            "typography": {
                "default":
                    {
                        "fontFamily": "Arial",
                        "fontSize": "16px",
                        "fontWeight": "400",
                        "lineHeight": "1.5",
                        "letterSpacing": "0.5em"
                    }
            }
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }

        {
            let theme: Theme = serde_json::from_str(
                r##"{
"typography":{"noticeButton":{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},"button":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"title":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"text":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"caption":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"12px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"noticeTitle":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"15px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"14px","lineHeight":1.5,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.5,"letterSpacing":"-0.02em"}],"h3":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h1":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"36px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"body":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"16px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"noticeBold":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"14px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"notice":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"13px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":400,"fontSize":"18px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"h2":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"20px","lineHeight":1.2,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"28px","lineHeight":1.2,"letterSpacing":"-0.01em"}],"result":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"24px","lineHeight":1.2,"letterSpacing":"-0.02em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":700,"fontSize":"32px","lineHeight":1.2,"letterSpacing":"-0.02em"}],"resultPoint":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"24px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":800,"fontSize":"28px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultText":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"18px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":600,"fontSize":"22px","lineHeight":1.4,"letterSpacing":"-0.01em"}],"resultList":[{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"16px","lineHeight":1.4,"letterSpacing":"-0.01em"},null,null,null,{"fontFamily":"Pretendard","fontStyle":"normal","fontWeight":500,"fontSize":"20px","lineHeight":1.4,"letterSpacing":"-0.01em"}]}
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(theme);
        }
    }
}
