use wasm_bindgen::JsValue;

use super::{
    STATE,
    capabilities::{Call, Json, Resolver, Values},
};

pub(crate) const RESOLVER: Resolver<JsValue, js_sys::Function, js_sys::JsString> = Resolver {
    state: &STATE,
    values: Values {
        is_null: JsValue::is_null,
        is_undefined: JsValue::is_undefined,
        is_object: JsValue::is_object,
        as_bool: JsValue::as_bool,
        as_string: JsValue::as_string,
        string: JsValue::from_str,
    },
    call: Call {
        invoke: js_sys::Function::call2,
        receiver: JsValue::NULL,
    },
    get: js_sys::Reflect::get,
    json: Json {
        stringify: js_sys::JSON::stringify,
        value: <js_sys::JsString as AsRef<JsValue>>::as_ref,
    },
};
