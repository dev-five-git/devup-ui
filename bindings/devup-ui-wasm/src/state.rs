use wasm_bindgen::prelude::*;

/// Internal test boundary: clear the engine, including calling-thread caches.
/// Never call during extraction or a resolver callback, or between build environments.
pub fn reset_state_internal() {
    super::with_style_sheet_mut(|sheet| *sheet = sheet::StyleSheet::default());
    super::MODULE_RESOLVER.with_borrow_mut(|resolver| *resolver = None);
    css::test_state::reset_state_for_testing();
    extractor::test_state::reset_caches_for_testing();
}

/// Internal/testing-only reset, deliberately present in normal WASM artifacts.
#[wasm_bindgen(js_name = "resetStateForTesting")]
pub fn reset_state_for_testing() {
    reset_state_internal();
}

#[cfg(test)]
pub(crate) struct TestStateGuard {
    sheet: sheet::StyleSheet,
    resolver: Option<js_sys::Function>,
    _engine: extractor::test_state::TestStateGuard,
}

#[cfg(test)]
impl TestStateGuard {
    pub(crate) fn new() -> Self {
        Self {
            sheet: super::with_style_sheet_mut(std::mem::take),
            resolver: super::MODULE_RESOLVER.with_borrow_mut(Option::take),
            _engine: extractor::test_state::TestStateGuard::new(),
        }
    }
}

#[cfg(test)]
impl Drop for TestStateGuard {
    fn drop(&mut self) {
        super::with_style_sheet_mut(|sheet| *sheet = std::mem::take(&mut self.sheet));
        super::MODULE_RESOLVER.with_borrow_mut(|resolver| *resolver = self.resolver.take());
    }
}
