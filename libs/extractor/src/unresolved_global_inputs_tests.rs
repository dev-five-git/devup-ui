mod boundaries;
mod coverage_edges;
mod fonts;
mod function_values;
mod functional_empty;
mod imports;
mod optional_params;
mod params;
mod positives;

use crate::{ExtractOption, ExtractOutput, dead_properties_test_utils::position, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};

fn extract_fixture(
    source: &str,
    single_css: bool,
) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    reset_class_map();
    reset_file_map();
    extract(
        "structural.tsx",
        source,
        ExtractOption {
            single_css,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
}

pub(super) fn rejects_at(source: &str, needle: &str, single_css: bool) {
    // Given an original-source marker, not its containing call or object.
    let offset = position(source, needle);
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit_once('\n')
        .map_or(before, |(_, tail)| tail)
        .chars()
        .count()
        + 1;
    // When the real extraction pipeline processes the structural input.
    let error = crate::dead_properties_test_utils::failure(extract_fixture(source, single_css));
    // Then both the location and rejected expression survive preprocessing.
    let (location, message) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("structural.tsx:{line}:{column}"));
    let (_, rejected) = message
        .split_once(" cannot use `")
        .unwrap_or_else(|| panic!("unexpected diagnostic: {message}"));
    let (code, requirement) = rejected
        .split_once("` at build time: ")
        .unwrap_or_else(|| panic!("missing requirement: {rejected}"));
    assert_eq!(code, needle);
    assert_ne!(requirement, "");
}
