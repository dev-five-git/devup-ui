use super::Check;
use crate::stylex::StylexFunction;
use crate::utils::build_time_error;

impl Check<'_> {
    pub(super) fn reject(&mut self, at: u32, code: &str, requirement: &str) {
        self.reject_error(at, build_time_error("StyleX API", code, requirement));
    }

    fn reject_error(&mut self, at: u32, message: String) {
        if self.diagnosed.iter().any(|(offset, _)| *offset == at) {
            return;
        }
        self.errors.push((at, message));
    }

    pub(super) fn reject_function(
        &mut self,
        function: &StylexFunction,
        site: (u32, &str),
        form: &str,
    ) {
        self.reject_error(
            site.0,
            build_time_error(
                &format!("stylex.{}", function.export_name()),
                site.1,
                &format!("unsupported {form}; call the API directly"),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::Bindings;

    #[rstest::rstest]
    #[case(4, 0)]
    #[case(5, 1)]
    fn function_diagnostic_is_suppressed_only_at_an_already_diagnosed_offset(
        #[case] previous_offset: u32,
        #[case] expected_count: usize,
    ) {
        // Given
        let bindings = Bindings::default();
        let diagnosed = [(previous_offset, String::from("previous diagnostic"))];
        let mut check = Check::new(&bindings, "@devup-ui/react", &diagnosed);
        // When
        check.reject_function(
            &StylexFunction::PositionTry,
            (4, "pt.call"),
            "invocation/binding member",
        );
        // Then
        assert_eq!(check.errors.len(), expected_count);
    }
}
