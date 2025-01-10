use extractor::{extract, ExtractOption, ExtractStyleValue};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
struct InExtractStyle(ExtractStyleValue);
#[wasm_bindgen]
pub struct ExtractOutput {
    code: String,
    styles: Vec<ExtractStyleValue>,
}
#[wasm_bindgen]
impl ExtractOutput {
    // #[wasm_bindgen(constructor)]
    // pub fn new(code: String, styles: Vec<ExtractStyle>) -> ExtractOutput {
    //     ExtractOutput { code, styles }
    // }

    pub fn code(&self) -> String {
        self.code.clone()
    }

    pub fn styles(&self) -> Vec<InExtractStyle> {
        self.styles
            .iter()
            .map(|style| InExtractStyle(style.clone()))
            .collect()
    }
}

#[wasm_bindgen(js_name = "codeExtract")]
pub fn code_extract(filename: &str, code: &str, package: &str) -> ExtractOutput {
    let output = extract(
        filename,
        code,
        ExtractOption {
            package: package.to_string(),
        },
    )
    .unwrap();
    ExtractOutput {
        code: output.code,
        styles: output.styles,
    }
}
