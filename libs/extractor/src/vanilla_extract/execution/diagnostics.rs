use boa_engine::Context;

pub(super) struct Diagnostics<'a> {
    pub run: &'a crate::module_loader::Script,
    pub operations: &'a crate::module_loader::operations::Operations,
    pub instrumented: &'a crate::evaluation_sandbox::Instrumented,
    pub filename: &'a str,
    pub required: bool,
}

impl Diagnostics<'_> {
    pub(super) fn explain(
        &self,
        failure: crate::evaluation_sandbox::Failure,
        context: &Context,
    ) -> String {
        let required = self
            .required
            .then(|| {
                crate::ordinary_ve::execution::diagnostics::required(
                    &failure,
                    self.run,
                    self.operations,
                )
            })
            .flatten();
        let message = crate::vanilla_extract::sandbox_error(
            failure,
            self.run,
            |error| {
                self.operations
                    .explain(&self.instrumented.explain(error), context)
            },
            self.filename,
        );
        match required {
            Some(required) => format!("{message}\n{required}"),
            None => message,
        }
    }
}
