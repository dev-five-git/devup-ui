use std::{cell::RefCell, collections::BTreeMap};

use js_sys::{Array, Error, Function, Object, PropertyDescriptor, TypeError, WeakMap};
use wasm_bindgen::JsValue;

use super::{
    BANK, ERRORS,
    access::Access,
    checkpoint::HostCheckpoint,
    error_capabilities::Transport,
    interpreter::{Call, Context, Decoder, Engine, Owner},
};

thread_local! {
    static CAPTURES: RefCell<BTreeMap<u64, Function>> = const { RefCell::new(BTreeMap::new()) };
}

type JsErrors = Transport<JsValue, Object, Array, PropertyDescriptor, Error, WeakMap>;

const OWNER: Owner<'static, JsErrors, Function, TypeError> = Owner {
    bank: Access { store: &BANK },
    callbacks: &CAPTURES,
    errors: ERRORS,
    call: Call {
        invoke: Function::call0,
        receiver: JsValue::UNDEFINED,
    },
    decoder: Decoder {
        get: js_sys::Reflect::get,
        string: JsValue::from_str,
        as_string: JsValue::as_string,
        type_error: TypeError::new,
    },
    engine: Engine {
        reset: crate::reset_build_state_internal,
        restore: HostCheckpoint::restore,
    },
    context: Context {
        retain_importer: crate::resolver::retain_importer,
        attach_cause: crate::resolver::attach_cause,
        clear_cause: crate::resolver::clear_cause,
    },
};

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerCreate, skip_typescript)]
pub fn create(capture: Function) -> Result<u64, JsValue> {
    OWNER.create(capture)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerReject, skip_typescript)]
pub fn reject() -> Result<(), JsValue> {
    OWNER.reject()
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerBegin, skip_typescript)]
pub fn begin(id: u64) -> Result<(), JsValue> {
    OWNER.begin(id)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerConfigured, skip_typescript)]
pub fn configured() -> Result<(), JsValue> {
    OWNER.configured()
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerFinish, skip_typescript)]
pub fn finish() -> Result<(), JsValue> {
    OWNER.finish()
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerEnterExport, skip_typescript)]
pub fn enter(opcode: u32) -> Result<u64, JsValue> {
    OWNER.enter(opcode)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerLeaveExport, skip_typescript)]
pub fn leave(id: u64) -> Result<(), JsValue> {
    OWNER.leave(id)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerDispose, skip_typescript)]
pub fn dispose(id: u64) -> Result<(), JsValue> {
    OWNER.dispose(id)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerClear, skip_typescript)]
pub fn clear(id: u64) -> Result<(), JsValue> {
    OWNER.clear(id)
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerDrain, skip_typescript)]
pub fn drain() -> Result<(), JsValue> {
    OWNER.drain()
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerDisposed, skip_typescript)]
pub fn disposed(id: u64) -> Result<bool, JsValue> {
    OWNER.disposed(id)
}
