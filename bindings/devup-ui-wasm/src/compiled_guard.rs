#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

pub use extractor::compiled_guard::compiled_guard_facts_internal;

/// Collect ordered guard facts from final JavaScript without executing it.
///
/// # Errors
/// Returns a JavaScript error with located final-output parser diagnostics.
#[wasm_bindgen(js_name = compiledGuardFacts)]
#[cfg(target_arch = "wasm32")]
pub fn compiled_guard_facts(code: &str, observe: bool) -> Result<String, JsValue> {
    compiled_guard_facts_internal(code, observe)
        .map_err(|error| js_sys::Error::new(&error.to_string()).into())
}
