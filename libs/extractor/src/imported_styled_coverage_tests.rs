use super::{Reader, written_name};
use crate::ExtractOption;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

#[test]
fn imported_evaluation_when_source_is_unparseable_returns_no_computed_module() {
    // Given
    let allocator = Allocator::default();
    let option = ExtractOption::default();
    let resolver = |_: &str, _: &str| None;
    let reader = Reader {
        option: &option,
        resolver: &resolver,
        allocator: &allocator,
        modules: FxHashMap::default(),
        reading: Vec::new(),
        dependencies: BTreeSet::new(),
    };
    // When
    let result = reader.evaluate_values(
        "/w27/broken.tsx",
        "export const Broken = ;",
        &crate::imported_constants::Unknown::default(),
    );
    // Then
    assert_eq!(result, None);
}

#[rstest]
#[case("Child", Some("Child"))]
#[case("UI.Child", Some("UI.Child"))]
#[case("UI['Child']", Some("UI.Child"))]
#[case("getUI().Child", None)]
#[case("UI.Nested.Child", None)]
#[case("UI[key]", None)]
fn imported_name_when_member_is_not_exact_does_not_select_a_definition(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .map_err(|error| format!("{error:?}"))?;
    // When
    let name = written_name(&expression);
    // Then
    assert_eq!(name.as_deref(), expected);
    Ok(())
}
