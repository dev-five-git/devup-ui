use css::class_map::{get_class_map, set_class_map};
use css::file_map::{get_file_map, get_filename_by_file_num, set_file_map};
use extractor::extract_style::ExtractStyleProperty;
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::extract_style::style_property::StyleProperty;
use extractor::{ExtractOption, extract};
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
    default_collected: bool,
    split_css: bool,
    filename: String,
    css_file: String,
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
    pub fn css_file(&self) -> String {
        self.css_file.clone()
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
                    let (cls, _) = match style.extract(if self.split_css {
                        Some(self.filename.as_str())
                    } else {
                        None
                    }) {
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
                        if self.split_css {
                            Some(self.filename.as_str())
                        } else {
                            None
                        },
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    let (cls, variable) = match style.extract(if self.split_css {
                        Some(self.filename.as_str())
                    } else {
                        None
                    }) {
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
                        if self.split_css {
                            Some(self.filename.as_str())
                        } else {
                            None
                        },
                    ) {
                        collected = true;
                    }
                }

                ExtractStyleValue::Keyframes(keyframes) => {
                    if sheet.add_keyframes(
                        &keyframes
                            .extract(if self.split_css {
                                Some(self.filename.as_str())
                            } else {
                                None
                            })
                            .to_string(),
                        keyframes
                            .keyframes
                            .iter()
                            .map(|(key, value)| {
                                (
                                    key.clone(),
                                    value
                                        .iter()
                                        .map(|style| {
                                            (
                                                style.property().to_string(),
                                                style.value().to_string(),
                                            )
                                        })
                                        .collect::<Vec<(String, String)>>(),
                                )
                            })
                            .collect(),
                        if self.split_css {
                            Some(self.filename.as_str())
                        } else {
                            None
                        },
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
                ExtractStyleValue::FontFace(font) => {
                    sheet.add_font_face(&font.file, &font.properties);
                }
            }
        }

        if !collected && !self.default_collected {
            return None;
        }

        Some(sheet.create_css(if self.split_css {
            Some(self.filename.as_str())
        } else {
            None
        }))
    }
}

#[wasm_bindgen(js_name = "setDebug")]
pub fn set_debug(debug: bool) {
    css::debug::set_debug(debug);
}

#[wasm_bindgen(js_name = "isDebug")]
pub fn is_debug() -> bool {
    css::debug::is_debug()
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

#[wasm_bindgen(js_name = "importFileMap")]
pub fn import_file_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_file_map(
        serde_wasm_bindgen::from_value(sheet_object)
            .map_err(|e| JsValue::from_str(e.to_string().as_str()))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportFileMap")]
pub fn export_file_map() -> Result<String, JsValue> {
    serde_json::to_string(&get_file_map()).map_err(|e| JsValue::from_str(e.to_string().as_str()))
}

#[wasm_bindgen(js_name = "codeExtract")]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    split_css: bool,
) -> Result<Output, JsValue> {
    let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();

    match extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
            css_dir,
            split_css,
        },
    ) {
        Ok(output) => Ok(Output {
            code: output.code,
            styles: output.styles,
            map: output.map,
            default_collected: sheet.rm_global_css(filename),
            split_css,
            filename: filename.to_string(),
            css_file: output.css_file,
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
pub fn get_css(file_num: Option<usize>) -> Result<String, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    Ok(sheet.create_css(file_num.map(|num| get_filename_by_file_num(num)).as_deref()))
}

#[wasm_bindgen(js_name = "getThemeInterface")]
pub fn get_theme_interface(
    package_name: &str,
    color_interface_name: &str,
    typography_interface_name: &str,
    theme_interface_name: &str,
) -> String {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    sheet.create_interface(
        package_name,
        color_interface_name,
        typography_interface_name,
        theme_interface_name,
    )
}
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;
    use sheet::theme::{ColorTheme, Theme, Typography};

    #[test]
    #[serial]
    fn test_code_extract() {
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        assert_eq!(get_css(None).unwrap(), "");

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

        assert_debug_snapshot!(get_css(None).unwrap());
    }

    #[test]
    #[serial]
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
            assert_eq!(theme.breakpoints, vec![0, 480, 768, 992, 1280, 1600]);
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

    #[test]
    #[serial]
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

    #[test]
    #[serial]
    fn test_get_theme_interface() {
        let sheet = StyleSheet::default();
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            ""
        );

        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("dark", color_theme);
        GLOBAL_STYLE_SHEET.lock().unwrap().set_theme(theme);
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{$primary:null;}interface TypographyInterface{}interface ThemeInterface{dark:null;}}"
        );

        // test wrong case
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
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{[`$(primary)`]:null;}interface TypographyInterface{[`prim\\`\\`ary`]:null;}interface ThemeInterface{dark:null;}}"
        );
    }

    #[test]
    #[serial]
    fn test_debug() {
        assert!(!is_debug());
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }

    #[test]
    #[serial]
    fn test_default_theme() {
        let mut theme = Theme::default();
        theme.add_color_theme("light", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());
        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("light".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("default", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("default".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("dark".to_string()));
    }
}
