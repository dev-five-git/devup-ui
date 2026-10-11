/// Native programmer contract: ordinary extraction never administers build state.
/// Real linkage activation will replace this panic with a recoverable typed error.
pub(crate) fn with_administration<R>(operation: &str, build: impl FnOnce() -> R) -> R {
    css::admission::with_admission(|| {
        css::admission::assert_administration_allowed(operation);
        build()
    })
}
