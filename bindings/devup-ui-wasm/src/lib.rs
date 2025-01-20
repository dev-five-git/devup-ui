use extractor::extract_style::ExtractStyleValue;
use extractor::{extract, ExtractOption, StyleProperty};
use js_sys::{Object, Reflect};
use once_cell::sync::Lazy;
use sheet::theme::{ColorTheme, Theme, Typography};
use sheet::StyleSheet;
use std::collections::HashSet;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static GLOBAL_STYLE_SHEET: Lazy<Mutex<StyleSheet>> = Lazy::new(|| Mutex::new(StyleSheet::new()));

#[wasm_bindgen]
pub struct Output {
    code: String,
    styles: Vec<ExtractStyleValue>,
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &JsValue);
}

#[wasm_bindgen]
impl Output {
    /// Get the code
    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    /// Get the css
    #[wasm_bindgen(getter)]
    pub fn css(&self) -> Option<String> {
        let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
        let mut collected = false;
        for style in self.styles.iter() {
            let (cls, variable) = match style.extract() {
                StyleProperty::ClassName(cls) => (cls, None),
                StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                } => (class_name, Some(variable_name)),
            };
            match style {
                ExtractStyleValue::Static(st) => {
                    if sheet.add_property(
                        &cls,
                        st.property(),
                        st.level(),
                        st.value(),
                        st.selector(),
                        st.basic(),
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    if sheet.add_property(
                        &cls,
                        dy.property(),
                        dy.level(),
                        &format!("var({})", variable.unwrap()),
                        dy.selector(),
                        false,
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Css(cs) => {
                    if sheet.add_css(&cls, &cs.css) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Typography(_) => {}
            }
        }
        if !collected {
            return None;
        }
        Some(sheet.create_css())
    }
}

#[wasm_bindgen(js_name = "codeExtract")]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_file: &str,
) -> Result<Output, JsValue> {
    match extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
            css_file: Some(css_file.to_string()),
        },
    ) {
        Ok(output) => Ok(Output {
            code: if output.styles.is_empty() {
                code.to_string()
            } else {
                output.code
            },
            styles: output.styles,
        }),
        Err(error) => Err(JsValue::from_str(error.to_string().as_str())),
    }
}
pub fn object_to_typography(obj: Object, level: u8) -> Result<Typography, JsValue> {
    Ok(Typography::new(
        Reflect::get(&obj, &JsValue::from_str("fontFamily"))
            .map(|v| v.as_string())
            .unwrap_or(None),
        Reflect::get(&obj, &JsValue::from_str("fontSize"))
            .map(|v| v.as_string())
            .unwrap_or(None),
        Reflect::get(&obj, &JsValue::from_str("fontWeight"))
            .map(|v| v.as_string())
            .unwrap_or(None),
        Reflect::get(&obj, &JsValue::from_str("lineHeight"))
            .map(|v| v.as_string())
            .unwrap_or(None),
        Reflect::get(&obj, &JsValue::from_str("letterSpacing"))
            .map(|v| v.as_string())
            .unwrap_or(None),
        level,
    ))
}

fn theme_object_to_hashmap(js_value: JsValue) -> Result<Theme, JsValue> {
    let mut theme = Theme::new();

    if let Ok(obj) = js_value.dyn_into::<Object>() {
        // get colors
        if let Some(colors_obj) = Reflect::get(&obj, &JsValue::from_str("colors"))
            .ok()
            .and_then(|v| v.dyn_into::<Object>().ok())
        {
            for entry in Object::entries(&colors_obj).into_iter() {
                if let (Ok(key), Ok(value)) = (
                    Reflect::get(&entry, &JsValue::from_f64(0f64)),
                    Reflect::get(&entry, &JsValue::from_f64(1f64)),
                ) {
                    if let (Some(key_str), Some(theme_value)) =
                        (key.as_string(), value.dyn_into::<Object>().ok())
                    {
                        let mut color_theme = ColorTheme::new();
                        for var_entry in Object::entries(&theme_value).into_iter() {
                            if let (Ok(var_key), Ok(var_value)) = (
                                Reflect::get(&var_entry, &JsValue::from_f64(0f64)),
                                Reflect::get(&var_entry, &JsValue::from_f64(1f64)),
                            ) {
                                if let (Some(var_key_str), Some(var_value_str)) =
                                    (var_key.as_string(), var_value.as_string())
                                {
                                    color_theme.add_color(&var_key_str, &var_value_str);
                                } else {
                                    return Err(JsValue::from_str(
                                        "Failed to get key and value from the theme object",
                                    ));
                                }
                            }
                        }
                        theme.colors.add_theme(&key_str, color_theme);
                    }
                }
            }
        }

        if let Some(typography_obj) = Reflect::get(&obj, &JsValue::from_str("typography"))
            .ok()
            .and_then(|v| v.dyn_into::<Object>().ok())
        {
            for entry in Object::entries(&typography_obj).into_iter() {
                if let (Ok(key), Ok(value)) = (
                    Reflect::get(&entry, &JsValue::from_f64(0f64)),
                    Reflect::get(&entry, &JsValue::from_f64(1f64)),
                ) {
                    if let (Some(key_str), Some(typo_value)) =
                        (key.as_string(), value.dyn_into::<Object>().ok())
                    {
                        let mut typo_vec = vec![];
                        if typo_value.is_array() {
                            if let Ok(typo_arr) = typo_value.dyn_into::<js_sys::Array>() {
                                for i in 0..typo_arr.length() {
                                    if let Ok(typo_obj) = typo_arr.get(i).dyn_into::<Object>() {
                                        typo_vec.push(object_to_typography(typo_obj, i as u8)?);
                                    }
                                }
                            }
                        } else if typo_value.is_object() && !typo_value.is_null() {
                            if let Ok(typo_obj) = typo_value.dyn_into::<Object>() {
                                typo_vec.push(object_to_typography(typo_obj, 0)?);
                            }
                        }
                        theme.typography.insert(key_str, typo_vec);
                    }
                }
            }
        }
    } else {
        return Err(JsValue::from_str(
            "Failed to convert the provided object to a hashmap",
        ));
    }
    Ok(theme)
}

#[wasm_bindgen(js_name = "registerTheme")]
pub fn register_theme(theme_object: JsValue) -> Result<(), JsValue> {
    let theme_object = theme_object_to_hashmap(theme_object)?;
    let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    sheet.set_theme(theme_object);
    Ok(())
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
) -> Result<String, JsValue> {
    let sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    let mut color_keys = HashSet::new();
    let mut typography_keys = HashSet::new();
    for color_theme in sheet.theme.colors.themes.values() {
        color_theme.keys().for_each(|key| {
            color_keys.insert(key.clone());
        });
    }
    sheet.theme.typography.keys().for_each(|key| {
        typography_keys.insert(key.clone());
    });

    if color_keys.is_empty() && typography_keys.is_empty() {
        Ok("".to_string())
    } else {
        Ok(format!(
            "import \"{}\";declare module \"{}\"{{interface {} {{{}}}interface {} {{{}}}}}",
            package_name,
            package_name,
            color_interface_name,
            color_keys
                .into_iter()
                .map(|key| format!("${}:null;", key))
                .collect::<Vec<String>>()
                .join(""),
            typography_interface_name,
            typography_keys
                .into_iter()
                .map(|key| format!("{}:null;", key))
                .collect::<Vec<String>>()
                .join("")
        ))
    }
}
