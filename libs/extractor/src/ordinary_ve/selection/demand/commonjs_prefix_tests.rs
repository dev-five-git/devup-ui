use rstest::rstest;

use super::test_support::{TestResult, path, selected, span};

#[rstest]
#[case("module.notExports[key].space=require('./ignored');")]
#[case("module['notExports'][key].space=require('./ignored');")]
#[case("module.notExports[key].exports.space=require('./ignored');")]
fn module_assignment_is_ignored_when_known_prefix_is_unrelated(#[case] source: &str) -> TestResult {
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
#[case("module[key].exports.space=require('./ignored');")]
#[case("module[key].space=require('./ignored');")]
#[case("module[key]['exports'][other].space=require('./ignored');")]
fn module_assignment_is_ignored_when_carrier_prefix_is_unproved(
    #[case] source: &str,
) -> TestResult {
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
#[case("module['exports'][key].space=8")]
#[case("module.exports[first][second].space=8")]
fn path_error_keeps_original_span_when_proven_carrier_has_unknown_segments(
    #[case] assignment: &str,
) -> TestResult {
    // Given
    let source = format!("// 한글\r\n  {assignment};");
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
