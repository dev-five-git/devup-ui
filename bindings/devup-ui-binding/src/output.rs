use napi::bindgen_prelude::{Either, Undefined};
use napi::{Env, Result};
use napi_derive::napi;

use crate::value::binding_error;

/// Materialized native output with an explicitly consumable payload.
#[napi(js_name = "Output")]
#[derive(Default)]
pub struct NativeOutput(pub(crate) Option<devup_ui_binding_core::Output>);
impl NativeOutput {
    fn live(&self, env: Env) -> Result<&devup_ui_binding_core::Output> {
        self.0
            .as_ref()
            .ok_or_else(|| binding_error(env, "null pointer passed to rust"))
    }
}
#[napi]
impl NativeOutput {
    /// Construct the same empty shell allowed by the generated WASM class.
    #[napi(constructor)]
    pub const fn new() -> Self {
        Self(None)
    }
    /// Consume the payload, rejecting a shell or a previously released output.
    #[napi]
    pub fn free(&mut self, env: Env) -> Result<()> {
        self.live(env)?;
        self.0 = None;
        Ok(())
    }
    /// Copy transformed source from a live output.
    #[napi(getter)]
    pub fn code(&self, env: Env) -> Result<String> {
        Ok(self.live(env)?.code.clone())
    }
    /// Copy the optional source map, using undefined for absence.
    #[napi(getter)]
    pub fn map(&self, env: Env) -> Result<Either<String, Undefined>> {
        Ok(self.live(env)?.map.clone().into())
    }
    /// Copy changed CSS, using undefined for absence.
    #[napi(getter)]
    pub fn css(&self, env: Env) -> Result<Either<String, Undefined>> {
        Ok(self.live(env)?.css.clone().into())
    }
    /// Copy the optional stylesheet filename.
    #[napi(getter, js_name = "cssFile")]
    pub fn css_file(&self, env: Env) -> Result<Either<String, Undefined>> {
        Ok(self.live(env)?.css_file.clone().into())
    }
    /// Return whether shared base CSS changed.
    #[napi(getter, js_name = "updatedBaseStyle")]
    pub fn updated_base_style(&self, env: Env) -> Result<bool> {
        Ok(self.live(env)?.updated_base_style)
    }
    /// Copy dependencies independently on every access.
    #[napi(getter)]
    pub fn dependencies(&self, env: Env) -> Result<Vec<String>> {
        Ok(self.live(env)?.dependencies.clone())
    }
}
