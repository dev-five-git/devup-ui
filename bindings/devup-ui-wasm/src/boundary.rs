use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn begin(slot: u32);
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn end();
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn error(value: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    pub(crate) fn fail(slot: u32) -> JsValue;
}

pub(crate) fn decode<T: DeserializeOwned>(value: JsValue, slot: u32) -> Result<T, JsValue> {
    begin(slot);
    match serde_wasm_bindgen::from_value(value) {
        Ok(value) => {
            end();
            Ok(value)
        }
        Err(cause) => Err(error(JsValue::from(cause))),
    }
}
