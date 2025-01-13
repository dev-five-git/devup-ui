use extractor::{extract, ExtractOption, ExtractStyleValue, StyleProperty};
use js_sys::{Object, Reflect};
use once_cell::sync::Lazy;
use sheet::theme::{ColorTheme, Theme};
use sheet::StyleSheet;
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
                        cls,
                        st.property.clone(),
                        st.level,
                        st.value.clone(),
                        st.selector.clone(),
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    if sheet.add_property(
                        cls,
                        dy.property.clone(),
                        dy.level,
                        format!("var({})", variable.unwrap()),
                        dy.selector.clone(),
                    ) {
                        collected = true;
                    }
                }
                ExtractStyleValue::Css(cs) => {
                    if sheet.add_css(cls, cs.css.clone()) {
                        collected = true;
                    }
                }
            }
        }
        if !collected {
            return None;
        }
        Some(sheet.create_css(vec![0, 480, 768, 992, 1280]))
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
pub fn theme_object_to_hashmap(js_value: JsValue) -> Result<Theme, JsValue> {
    let mut theme = Theme::new();

    if let Some(obj) = js_value.dyn_into::<Object>().ok() {
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
                                    color_theme.add_color(var_key_str, var_value_str);
                                } else {
                                    return Err(JsValue::from_str(
                                        "Failed to get key and value from the theme object",
                                    ));
                                }
                            }
                        }
                        theme.colors.add_theme(key_str, color_theme);
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
    log(&theme_object);
    let theme_object = theme_object_to_hashmap(theme_object)?;
    let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
    sheet.set_theme(theme_object);
    Ok(())
}
