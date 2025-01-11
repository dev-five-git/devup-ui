use extractor::{extract, ExtractOption, ExtractStyleValue, StyleProperty};
use once_cell::sync::Lazy;
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
    fn log(s: &str);
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
        let mut collected = vec![];
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
                    if let Some(css) =
                        sheet.add_property(cls, st.property.clone(), st.value.clone())
                    {
                        collected.push(css);
                    }
                }
                ExtractStyleValue::Dynamic(dy) => {
                    if let Some(css) = sheet.add_property(
                        cls,
                        dy.property.clone(),
                        format!("var({})", variable.unwrap()),
                    ) {
                        collected.push(css);
                    }
                }
                ExtractStyleValue::Css(cs) => {
                    if let Some(css) = sheet.add_css(cls, cs.css.clone()) {
                        collected.push(css);
                    }
                }
            }
        }
        if collected.is_empty() {
            return None;
        }
        collected.push("".to_string());
        Some(collected.join("\n"))
    }
}

#[wasm_bindgen(js_name = "codeExtract", catch)]
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
