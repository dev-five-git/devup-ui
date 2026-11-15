pub(crate) fn report<E>(info: impl std::fmt::Display, console: fn(&str) -> Result<(), E>) {
    // Host console failure must not replace the aborting Rust panic.
    match console(&info.to_string()) {
        Ok(()) | Err(_) => {}
    }
}

/// Report the panic payload and Rust `file:line:column` before an aborting WASM trap.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn install_panic_hook() {
    #[wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = console, js_name = error, catch)]
        fn report_panic(message: &str) -> Result<(), wasm_bindgen::JsValue>;
    }
    std::panic::set_hook(Box::new(|info| report(info, report_panic)));
}
