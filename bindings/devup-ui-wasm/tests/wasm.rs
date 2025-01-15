use devup_ui_wasm::object_to_typography;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_object_to_typography() {
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("fontFamily"),
        &JsValue::from_str("Arial"),
    )
    .unwrap();
    Reflect::set(
        &obj,
        &JsValue::from_str("fontSize"),
        &JsValue::from_str("12px"),
    )
    .unwrap();
    Reflect::set(
        &obj,
        &JsValue::from_str("fontWeight"),
        &JsValue::from_str("bold"),
    )
    .unwrap();
    Reflect::set(
        &obj,
        &JsValue::from_str("lineHeight"),
        &JsValue::from_str("1.5"),
    )
    .unwrap();
    Reflect::set(
        &obj,
        &JsValue::from_str("letterSpacing"),
        &JsValue::from_str("1px"),
    )
    .unwrap();
    let typography = object_to_typography(obj, 0).unwrap();
    assert_eq!(typography.font_family, "Arial");
    assert_eq!(typography.font_size, "12px");
    assert_eq!(typography.font_weight, "bold");
    assert_eq!(typography.line_height, "1.5");
    assert_eq!(typography.letter_spacing, "1px");
    assert_eq!(typography.level, 0);
}
