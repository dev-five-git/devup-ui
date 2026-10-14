use super::*;

#[path = "cache_allocator_tests.rs"]
mod allocators;
#[path = "cache_naming_coverage_tests.rs"]
mod naming_coverage;
#[path = "cache_reset_tests.rs"]
mod resets;
#[path = "cache_seed_tests.rs"]
mod seeds;
#[path = "cache_typography_coverage_tests.rs"]
mod typography_coverage;

fn fresh() {
    reset_build_state_internal();
    set_debug(false);
    register_theme_internal(sheet::theme::Theme::default());
}

fn output(file: &str, source: &str, single: bool) -> (String, String, String) {
    let compiled = code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    with_style_sheet(|sheet| {
        (
            compiled.code(),
            sheet.create_css(None, false),
            sheet.create_css(Some(file), false),
        )
    })
}

fn companions(value: &serde_json::Value) {
    cache_restore::classes(Some(
        serde_json::from_value(value["classMap"].clone()).unwrap_or_else(|error| panic!("{error}")),
    ));
    cache_restore::files(Some(
        serde_json::from_value(value["fileMap"].clone()).unwrap_or_else(|error| panic!("{error}")),
    ));
}
