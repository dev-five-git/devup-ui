//! Body-free dependency wiring. The sole export is a mechanical forward.
use std::cell::LazyCell;

use js_sys::{Array, Error, Object, PropertyDescriptor, WeakMap};
use wasm_bindgen::{JsCast, JsValue};

use super::error_capabilities::{
    Arrays, Authentication, Errors, Importer, Properties, Transport, Values,
};

thread_local! {
    static FAULTS: LazyCell<WeakMap> = const { LazyCell::new(WeakMap::new) };
}

pub(crate) const ERRORS: Transport<JsValue, Object, Array, PropertyDescriptor, Error, WeakMap> =
    Transport {
        values: Values {
            undefined: JsValue::UNDEFINED,
            truth: JsValue::TRUE,
            is_object: JsValue::is_object,
            is_undefined: JsValue::is_undefined,
            object: JsCast::unchecked_ref,
            string: JsValue::from_str,
        },
        arrays: Arrays {
            new: Array::new,
            from: Array::from,
            push: Array::push,
            get: Array::get,
            set: Array::set,
            of4: Array::of4,
        },
        properties: Properties {
            descriptor: PropertyDescriptor::new_value,
            configurable: PropertyDescriptor::set_configurable,
            writable: PropertyDescriptor::set_writable,
            object: JsCast::unchecked_ref,
            define: Object::define_property,
        },
        errors: Errors {
            new: Error::new,
            name: Error::set_name,
        },
        authentication: Authentication {
            registry: &FAULTS,
            get: WeakMap::get,
            set: WeakMap::set,
        },
        importer: Importer {
            retain: crate::resolver::retain_importer,
            current: crate::resolver::importer,
        },
    };

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerFault, skip_typescript)]
pub fn details(value: &JsValue) -> JsValue {
    ERRORS.details(value)
}
