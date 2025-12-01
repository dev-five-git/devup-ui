use js_sys::{Object, Reflect};
use sheet::theme::Typography;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[allow(dead_code)]
#[wasm_bindgen_test]
fn test_object_to_typography() {
    let obj = Object::new();
    Reflect::set(&obj, &JsValue::from_str("fontFamily"), &JsValue::from_str("Arial")).unwrap();
    Reflect::set(&obj, &JsValue::from_str("fontSize"), &JsValue::from_str("12px")).unwrap();
    Reflect::set(&obj, &JsValue::from_str("fontWeight"), &JsValue::from_str("bold")).unwrap();
    Reflect::set(&obj, &JsValue::from_str("lineHeight"), &JsValue::from_str("1.5")).unwrap();
    Reflect::set(&obj, &JsValue::from_str("letterSpacing"), &JsValue::from_str("1px")).unwrap();
    let typography: Typography = serde_wasm_bindgen::from_value(JsValue::from(obj)).unwrap();
    assert_eq!(typography.font_family.unwrap(), "Arial");
    assert_eq!(typography.font_size.unwrap(), "12px");
    assert_eq!(typography.font_weight.unwrap(), "bold");
    assert_eq!(typography.line_height.unwrap(), "1.5");
    assert_eq!(typography.letter_spacing.unwrap(), "1px");
}
