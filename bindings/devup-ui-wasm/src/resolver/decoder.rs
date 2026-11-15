use extractor::{ModuleResolution, ResolvedModule};

use super::capabilities::Resolver;

impl<V: Clone, F, S> Resolver<V, F, S> {
    fn field(&self, module: &V, name: &str) -> Result<V, String> {
        (self.get)(module, &(self.values.string)(name)).map_err(|error| {
            let cause = self.diagnostic(&error);
            format!("reading resolver field `{name}`: {cause}")
        })
    }

    pub(crate) fn call(
        &self,
        resolver: &F,
        specifier: &str,
        importer: &str,
    ) -> Result<Option<ModuleResolution>, String> {
        self.state.with(|state| {
            let _scope = state.enter(importer);
            let module = (self.call.invoke)(
                resolver,
                &self.call.receiver,
                &(self.values.string)(specifier),
                &(self.values.string)(importer),
            )
            .map_err(|error| self.diagnostic(&error))?;
            if (self.values.is_null)(&module) || (self.values.is_undefined)(&module) {
                return Ok(None);
            }
            if !(self.values.is_object)(&module) {
                return Err("resolver result must be an object".to_string());
            }
            let ignored = self.field(&module, "ignored")?;
            if !(self.values.is_undefined)(&ignored) {
                (self.values.as_bool)(&ignored)
                    .filter(|value| *value)
                    .ok_or_else(|| {
                        "resolver field `ignored` must be true or undefined".to_string()
                    })?;
                return Ok(Some(ModuleResolution::Ignored));
            }
            let field = |name: &str| {
                (self.values.as_string)(&self.field(&module, name)?)
                    .ok_or_else(|| format!("resolver field `{name}` must be a string"))
            };
            let path = field("path")?;
            let code = field("code")?;
            let source_type = self.source_type(&self.field(&module, "sourceType")?)?;
            let resolved = ResolvedModule {
                path,
                code,
                source_type,
            };
            Ok(Some(ModuleResolution::Resolved(resolved)))
        })
    }
}
