//! Body-free function-item wiring, including serde's raw host conversion.
use js_sys::{Array, Error, Function, JsString, Object, PropertyDescriptor, WeakMap};
use wasm_bindgen::JsValue;

use super::{Conversion, Driver, Errors};
use crate::{resident_owner::Transport, resolver::RESOLVER};

type JsErrors = Transport<JsValue, Object, Array, PropertyDescriptor, Error, WeakMap>;
pub(crate) const DRIVER: Driver<
    JsValue,
    Function,
    JsString,
    serde_wasm_bindgen::Error,
    Error,
    JsErrors,
> = Driver {
    resolver: RESOLVER,
    installed: &crate::MODULE_RESOLVER,
    conversion: Conversion {
        aliases: serde_wasm_bindgen::from_value,
        strings: serde_wasm_bindgen::from_value,
        rows: serde_wasm_bindgen::from_value,
    },
    errors: Errors {
        new: Error::new,
        transport: crate::resident_owner::ERRORS,
        structural: Transport::error,
        attach: crate::resolver::attach_cause,
        fault: crate::resident_owner::check_fault,
    },
    export_active: crate::resident_owner::export_active,
};
