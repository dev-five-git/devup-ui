mod checks;
mod cleanup;

use super::Bindings;
use oxc_ast::ast::Program;

impl Bindings {
    /// Diagnose residual API reads before removing their consumed source bindings.
    pub(crate) fn finish_stylex(
        &self,
        program: &mut Program<'_>,
        package: &str,
        diagnosed: &[(u32, String)],
    ) -> Vec<(u32, String)> {
        let errors = checks::Check::new(self, package, diagnosed).errors(program);
        cleanup::remove(self, program, package);
        errors
    }
}
