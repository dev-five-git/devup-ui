use css::class_map::{get_class_map, set_class_map};
use css::file_map::{get_file_map, get_filename_by_file_num, set_file_map};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, ImportAlias, extract, has_devup_ui};
use once_cell::sync::Lazy;
use sheet::StyleSheet;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static GLOBAL_STYLE_SHEET: Lazy<Mutex<StyleSheet>> =
    Lazy::new(|| Mutex::new(StyleSheet::default()));

#[wasm_bindgen]
pub struct Output {
    code: String,
    map: Option<String>,
    css_file: Option<String>,
    updated_base_style: bool,
    css: Option<String>,
}
// #[wasm_bindgen]
// extern "C" {
//     #[wasm_bindgen(js_namespace = console)]
//     fn log(s: &JsValue);
//     #[wasm_bindgen(js_namespace = console, js_name = log)]
//     fn log_str(s: &str);
//     #[wasm_bindgen(js_namespace = console, js_name = time)]
//     fn time(s: &str);
//     #[wasm_bindgen(js_namespace = console, js_name = timeEnd)]
//     fn time_end(s: &str);
// }

#[wasm_bindgen]
impl Output {
    fn new(
        code: String,
        styles: HashSet<ExtractStyleValue>,
        map: Option<String>,
        single_css: bool,
        filename: String,
        css_file: Option<String>,
        import_main_css: bool,
    ) -> Self {
        let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
        let default_collected = sheet.rm_global_css(&filename, single_css);
        let (collected, updated_base_style) = sheet.update_styles(&styles, &filename, single_css);
        Self {
            code,
            map,
            css_file,
            updated_base_style: updated_base_style || default_collected,
            css: {
                if !collected && !default_collected {
                    None
                } else {
                    Some(sheet.create_css(
                        if !single_css { Some(&filename) } else { None },
                        import_main_css,
                    ))
                }
            },
        }
    }

    /// Get the code
    #[wasm_bindgen(getter, js_name = "code")]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter, js_name = "cssFile")]
    pub fn css_file(&self) -> Option<String> {
        self.css_file.clone()
    }

    #[wasm_bindgen(getter, js_name = "map")]
    pub fn map(&self) -> Option<String> {
        self.map.clone()
    }

    #[wasm_bindgen(getter, js_name = "updatedBaseStyle")]
    pub fn updated_base_style(&self) -> bool {
        self.updated_base_style
    }

    /// Get the css
    #[wasm_bindgen(getter, js_name = "css")]
    pub fn css(&self) -> Option<String> {
        self.css.clone()
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

/// Set the CSS class name prefix
///
/// # Example (Vite Config)
/// ```javascript
/// import init, { setPrefix, codeExtract } from 'devup-ui-wasm';
///
/// export default {
///   plugins: [
///     {
///       name: 'devup-ui',
///       apply: 'pre',
///       async configResolved() {
///         await init();
///         setPrefix('du-'); // Set prefix to 'du-'
///       },
///       // ... other plugin code
///     }
///   ]
/// }
/// ```
///
/// # Example (Next.js Plugin)
/// ```typescript
/// import init, { setPrefix } from 'devup-ui-wasm';
///
/// const withDevupUI = (nextConfig) => {
///   return {
///     ...nextConfig,
///     webpack: (config, options) => {
///       if (!options.isServer && !global.devupUIInitialized) {
///         init().then(() => {
///           setPrefix('du-');
///           global.devupUIInitialized = true;
///         });
///       }
///       return config;
///     }
///   };
/// };
/// ```
#[wasm_bindgen(js_name = "setPrefix")]
pub fn set_prefix(prefix: Option<String>) {
    css::set_prefix(prefix);
}

#[wasm_bindgen(js_name = "getPrefix")]
pub fn get_prefix() -> Option<String> {
    css::get_prefix()
}

/// Internal function to import a StyleSheet (testable without JsValue)
pub fn import_sheet_internal(sheet: StyleSheet) {
    *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
}

#[wasm_bindgen(js_name = "importSheet")]
pub fn import_sheet(sheet_object: JsValue) -> Result<(), JsValue> {
    let sheet: StyleSheet = serde_wasm_bindgen::from_value(sheet_object)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    import_sheet_internal(sheet);
    Ok(())
}

/// Internal function to export StyleSheet as JSON string (testable without JsValue)
pub fn export_sheet_internal() -> Result<String, String> {
    serde_json::to_string(&*GLOBAL_STYLE_SHEET.lock().unwrap()).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "exportSheet")]
pub fn export_sheet() -> Result<String, JsValue> {
    export_sheet_internal().map_err(|e| JsValue::from_str(&e))
}

/// Internal function to export class map as JSON string (testable without JsValue)
pub fn export_class_map_internal() -> Result<String, String> {
    serde_json::to_string(&get_class_map()).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "importClassMap")]
pub fn import_class_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_class_map(
        serde_wasm_bindgen::from_value(sheet_object)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportClassMap")]
pub fn export_class_map() -> Result<String, JsValue> {
    export_class_map_internal().map_err(|e| JsValue::from_str(&e))
}

/// Internal function to export file map as JSON string (testable without JsValue)
pub fn export_file_map_internal() -> Result<String, String> {
    serde_json::to_string(&get_file_map()).map_err(|e| e.to_string())
}

#[wasm_bindgen(js_name = "importFileMap")]
pub fn import_file_map(sheet_object: JsValue) -> Result<(), JsValue> {
    set_file_map(
        serde_wasm_bindgen::from_value(sheet_object)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportFileMap")]
pub fn export_file_map() -> Result<String, JsValue> {
    export_file_map_internal().map_err(|e| JsValue::from_str(&e))
}

/// Internal function to extract code (testable without JsValue)
#[allow(clippy::too_many_arguments)]
pub fn code_extract_internal(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
) -> Result<Output, String> {
    match extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
            css_dir,
            single_css,
            import_main_css: import_main_css_in_code,
            import_aliases,
        },
    ) {
        Ok(output) => Ok(Output::new(
            output.code,
            output.styles,
            output.map,
            single_css,
            filename.to_string(),
            output.css_file,
            import_main_css_in_css,
        )),
        Err(error) => Err(error.to_string()),
    }
}

#[wasm_bindgen(js_name = "codeExtract")]
#[allow(clippy::too_many_arguments)]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: JsValue,
) -> Result<Output, JsValue> {
    // Deserialize import_aliases from JsValue
    // Format: { "package": "namedExport" } or { "package": null } for named exports
    let aliases: HashMap<String, Option<String>> =
        serde_wasm_bindgen::from_value(import_aliases).unwrap_or_default();

    // Convert to ImportAlias enum
    let import_aliases: HashMap<String, ImportAlias> = aliases
        .into_iter()
        .map(|(k, v)| {
            let alias = match v {
                Some(name) => ImportAlias::DefaultToNamed(name),
                None => ImportAlias::NamedToNamed,
            };
            (k, alias)
        })
        .collect();

    code_extract_internal(
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
    )
    .map_err(|e| JsValue::from_str(&e))
}

/// Internal function to register theme (testable without JsValue)
pub fn register_theme_internal(theme: sheet::theme::Theme) {
    GLOBAL_STYLE_SHEET.lock().unwrap().set_theme(theme);
}

#[wasm_bindgen(js_name = "registerTheme")]
pub fn register_theme(theme_object: JsValue) -> Result<(), JsValue> {
    let theme: sheet::theme::Theme = serde_wasm_bindgen::from_value(theme_object)
        .map_err(|e| JsValue::from_str(e.to_string().as_str()))?;
    register_theme_internal(theme);
    Ok(())
}

#[wasm_bindgen(js_name = "getDefaultTheme")]
pub fn get_default_theme() -> Result<Option<String>, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    Ok(sheet.theme.get_default_theme())
}

#[wasm_bindgen(js_name = "getCss")]
pub fn get_css(file_num: Option<usize>, import_main_css: bool) -> Result<String, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    Ok(sheet.create_css(
        file_num.map(get_filename_by_file_num).as_deref(),
        import_main_css,
    ))
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

#[wasm_bindgen(js_name = "hasDevupUI")]
pub fn has_devup_ui_wasm(filename: &str, code: &str, package: &str) -> bool {
    has_devup_ui(filename, code, package)
}

#[cfg(test)]
mod tests {
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
        assert_eq!(
            get_css(None, false).unwrap().split("*/").nth(1).unwrap(),
            ""
        );

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

        assert_debug_snapshot!(get_css(None, false).unwrap().split("*/").nth(1).unwrap());
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
            "import \"package\";declare module \"package\"{interface ColorInterface{$primary:null}interface TypographyInterface{}interface ThemeInterface{dark:null}}"
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
            "import \"package\";declare module \"package\"{interface ColorInterface{[`$(primary)`]:null}interface TypographyInterface{[`prim\\`\\`ary`]:null}interface ThemeInterface{dark:null}}"
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
    fn test_prefix() {
        assert_eq!(get_prefix(), None);
        set_prefix(Some("du-".to_string()));
        assert_eq!(get_prefix(), Some("du-".to_string()));
        set_prefix(None);
        assert_eq!(get_prefix(), None);
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

    #[test]
    #[serial]
    fn test_output_new_and_getters() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box bg="red" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: false,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code.clone(),
            result.styles,
            Some("//# sourceMappingURL=test".to_string()),
            false,
            "test.tsx".to_string(),
            Some("devup-ui-0.css".to_string()),
            false,
        );

        // Test getters
        assert!(!output.code().is_empty());
        assert_eq!(output.css_file(), Some("devup-ui-0.css".to_string()));
        assert_eq!(output.map(), Some("//# sourceMappingURL=test".to_string()));
        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_updated_base_style() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create output with empty styles
        let styles = HashSet::new();
        let output = Output::new(
            "code".to_string(),
            styles,
            None,
            true,
            "test.tsx".to_string(),
            None,
            false,
        );

        // Test updated_base_style getter
        let _ = output.updated_base_style();
        assert!(output.css().is_none()); // No styles = no CSS
    }

    #[test]
    #[serial]
    fn test_has_devup_ui_wasm_function() {
        // Test positive case
        assert!(has_devup_ui_wasm(
            "test.tsx",
            "import { Box } from '@devup-ui/react';",
            "@devup-ui/react"
        ));

        // Test negative case
        assert!(!has_devup_ui_wasm(
            "test.tsx",
            "const x = 1;",
            "@devup-ui/react"
        ));

        // Test invalid extension
        assert!(!has_devup_ui_wasm(
            "test.invalid",
            "import { Box } from '@devup-ui/react';",
            "@devup-ui/react"
        ));
    }

    #[test]
    #[serial]
    fn test_output_single_css_mode() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles in single_css mode
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box color="blue" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: true,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code,
            result.styles,
            None,
            true, // single_css = true
            "test.tsx".to_string(),
            Some("devup-ui.css".to_string()),
            true, // import_main_css = true
        );

        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_with_global_css_removal() {
        // Reset global state
        let mut sheet = StyleSheet::default();

        // Add some global CSS first
        sheet.add_property(
            "test.tsx",
            "margin",
            0,
            "0",
            Some(&css::style_selector::StyleSelector::Global(
                "body".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );

        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        css::class_map::reset_class_map();

        // Now create output which should trigger rm_global_css
        let styles = HashSet::new();
        let output = Output::new(
            "new code".to_string(),
            styles,
            None,
            false,
            "test.tsx".to_string(),
            None,
            false,
        );

        // The updated_base_style should be true because global CSS was removed
        assert!(output.updated_base_style());
    }

    #[test]
    #[serial]
    fn test_import_sheet_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create a custom sheet with a property
        let mut custom_sheet = StyleSheet::default();
        custom_sheet.add_property("custom.tsx", "color", 0, "red", None, Some(0), None);

        // Import the custom sheet
        import_sheet_internal(custom_sheet);

        // Verify the sheet was imported by exporting it
        let result = export_sheet_internal();
        assert!(result.is_ok());
        // The exported JSON should contain the property we added
        let json = result.unwrap();
        assert!(json.contains("color") || json.contains("red"));
    }

    #[test]
    #[serial]
    fn test_export_sheet_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Export the sheet
        let result = export_sheet_internal();
        assert!(result.is_ok());

        // The result should be valid JSON
        let json_str = result.unwrap();
        assert!(json_str.starts_with("{"));
        assert!(json_str.ends_with("}"));
    }

    #[test]
    #[serial]
    fn test_export_class_map_internal() {
        // Reset class map
        css::class_map::reset_class_map();

        // Export the class map
        let result = export_class_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert_eq!(json_str, "{}");
    }

    #[test]
    #[serial]
    fn test_export_file_map_internal() {
        // Reset file map
        css::file_map::reset_file_map();

        // Export the file map
        let result = export_file_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert!(json_str.starts_with("{") || json_str.starts_with("[]"));
    }

    #[test]
    #[serial]
    fn test_code_extract_internal_success() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test successful extraction
        let result = code_extract_internal(
            "test.tsx",
            r#"import {Box} from '@devup-ui/react'
<Box color="red" />"#,
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.code().is_empty());
    }

    #[test]
    #[serial]
    fn test_code_extract_internal_error() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test extraction with invalid file extension (should fail)
        let result = code_extract_internal(
            "test.invalid_extension", // Invalid extension will cause SourceType::from_path to fail
            "import { Box } from '@devup-ui/react'; <Box />",
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
        );

        assert!(result.is_err());
        if let Err(error) = result {
            assert!(!error.is_empty());
        }
    }

    #[test]
    #[serial]
    fn test_register_theme_internal() {
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Create and register a theme
        let mut theme = sheet::theme::Theme::default();
        let mut color_theme = sheet::theme::ColorTheme::default();
        color_theme.add_color("primary", "#ff0000");
        theme.add_color_theme("default", color_theme);

        register_theme_internal(theme);

        // Verify the theme was registered
        let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
        let default_theme = sheet.theme.get_default_theme();
        assert_eq!(default_theme, Some("default".to_string()));
    }
}
