use rstest::rstest;

use super::super::Demand;
use super::test_support::{TestResult, path, selected, span};

#[rstest]
#[case(concat!("exports = {", "space:8};"))]
#[case("factory().exports.space = 8;")]
#[case("other.exports.space = 8;")]
#[case("const exports={};exports.space=8;")]
#[case("const module={exports:{}};module['exports'].space=8;")]
#[case("function local(exports){exports.space=8;}")]
#[case("{let module={exports:{}};module.exports.space=8;}")]
fn no_export_is_selected_when_assignment_has_no_global_carrier(#[case] source: &str) -> TestResult {
    // Given
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![]);
    assert_eq!(view.forwarded, vec![]);
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[rstest]
#[case("exports.tokens.space={small:8,browser:window};")]
#[case("module.exports.tokens.space={small:8,browser:window};")]
#[case("module['exports'].tokens.space={small:8,browser:window};")]
#[case("module['exports']['tokens'].space={small:8,browser:window};")]
fn nested_export_is_narrowed_when_carrier_path_is_exact(#[case] source: &str) -> TestResult {
    // Given
    let demand = path(&["tokens", "space", "small"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, path(&["small"]))]);
    assert_eq!(view.omitted, vec![span(source, "browser:window")?]);
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[test]
fn sibling_assignment_is_unselected_when_only_space_is_demanded() -> TestResult {
    // Given
    let source = "exports.tokens.space=8;exports.tokens.browser=require('never');";
    let demand = path(&["tokens", "space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(
        view.units,
        vec![(span(source, "exports.tokens.space=8;")?, Demand::whole())]
    );
    assert_eq!(view.forwarded, vec![]);
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[rstest]
#[case("exports[key]=8")]
#[case("module.exports[key]=8")]
fn key_error_uses_original_assignment_span_when_final_key_is_unknown(
    #[case] assignment: &str,
) -> TestResult {
    // Given
    let source = format!("// 한글\r\n{assignment};");
    let demand = path(&["space"]);
    // When
    let view = selected(&source, &demand)?;
    // Then
    assert_eq!(view.units, vec![]);
    assert_eq!(
        view.errors,
        vec![(
            span(&source, assignment)?,
            "required CommonJS export needs an exact static key".to_string()
        )]
    );
    Ok(())
}

#[rstest]
#[case("exports[key].space=8")]
#[case("module.exports[key].space=8")]
fn path_error_uses_original_assignment_span_when_inner_key_is_unknown(
    #[case] assignment: &str,
) -> TestResult {
    // Given
    let source = format!("// 한글\r\n{assignment};");
    let demand = path(&["space"]);
    // When
    let view = selected(&source, &demand)?;
    // Then
    assert_eq!(view.units, vec![]);
    assert_eq!(
        view.errors,
        vec![(
            span(&source, assignment)?,
            "required CommonJS export needs an exact static path".to_string()
        )]
    );
    Ok(())
}

#[test]
fn module_assignment_is_ignored_when_member_is_not_exports() -> TestResult {
    // Given
    let source = "module.notExports.tokens.space=require('./ignored');";
    let demand = path(&["tokens", "space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![]);
    assert_eq!(view.forwarded, vec![]);
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[test]
fn default_demand_is_merged_when_assignment_replaces_whole_carrier() -> TestResult {
    // Given
    let source = "module['exports']={tokens:{space:8,browser:window},color:'red',unused:document};";
    let mut demand = path(&["default", "tokens", "space"]);
    demand.merge(&path(&["color"]));
    let mut expected = path(&["tokens", "space"]);
    expected.merge(&path(&["color"]));
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, expected)]);
    assert_eq!(
        view.omitted,
        vec![
            span(source, "browser:window")?,
            span(source, "unused:document")?
        ]
    );
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[test]
fn whole_default_observation_dominates_when_assignment_replaces_carrier() -> TestResult {
    // Given
    let source = "module.exports={space:8,browser:window};";
    let demand = path(&["default"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, Demand::whole())]);
    assert_eq!(view.omitted, vec![]);
    assert_eq!(view.errors, vec![]);
    Ok(())
}
