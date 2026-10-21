//! Live JS-only boundaries for the W74a coverage experiment.
use napi::{Env, Error, JsNumber, Result, Status, bindgen_prelude::Function};
use napi_derive::napi;

/// Converts through the pure core, then creates a real JS value using Env.
/// # Errors
/// Rejects negative input or a failed Node-API value creation.
#[napi]
pub fn converted(env: &Env, value: i32) -> Result<JsNumber<'_>> {
    let value = w74a_core::nonnegative(value)
        .map_err(|error| Error::new(Status::InvalidArg, error.to_string()))?;
    env.create_uint32(value)
}

/// Resolves synchronously; callback throws/conversion failures are cache misses.
#[must_use = "Use the resolved value or handle the cache miss"]
#[napi]
pub fn callback_probe(callback: Function<'_, (), i32>) -> Option<i32> {
    callback.call(()).ok()
}

/// Owns a native allocation until explicit free (or normal finalization).
#[napi]
pub struct Probe {
    value: Option<String>,
}

#[napi]
impl Probe {
    #[napi(constructor)]
    #[must_use = "Use the native probe instance"]
    pub fn new(value: String) -> Self {
        Self { value: Some(value) }
    }

    /// # Errors
    /// Rejects access after explicit free.
    #[napi(getter)]
    pub fn value(&self) -> Result<String> {
        match &self.value {
            Some(value) => Ok(value.clone()),
            None => Err(Error::new(Status::InvalidArg, "W74a probe is freed")),
        }
    }

    #[napi]
    pub fn free(&mut self) -> bool {
        self.value.take().is_some()
    }
}

/// Negative control must leave these executable body lines at zero hits.
#[must_use = "Use the control result"]
#[napi]
pub fn control_only(value: u32) -> u32 {
    let rotated = value.rotate_left(3); // W74a control sentinel A
    rotated ^ 0x74a // W74a control sentinel B
}
