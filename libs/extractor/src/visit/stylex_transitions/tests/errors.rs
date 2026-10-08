use super::reset;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("positionTry({color:'red'})", "2:34:")]
#[case("positionTry({'position-anchor':'--a'})", "2:34:")]
#[case("viewTransitionClass({ animationDuration:300})", "2:43:")]
#[case("viewTransitionClass({root:{}})", "2:42:")]
#[case("viewTransitionClass({ old:null})", "2:47:")]
#[case("viewTransitionClass({old:{opacity:runtime}})", "2:55:")]
#[case("positionTry({ top:runtime})", "2:39:")]
#[case("viewTransitionClass({old:{opacity:{ default:0}}})", "2:55:")]
#[case("viewTransitionClass({old:{...runtime}})", "2:47:")]
#[case("positionTry({[runtime]:1})", "2:35:")]
#[case("positionTry({get top(){return 1}})", "2:34:")]
#[case("viewTransitionClass({old(){return {}}})", "2:42:")]
#[case("viewTransitionClass({old:{get opacity(){return 1}}})", "2:47:")]
#[case("viewTransitionClass({old:{[runtime]:0}})", "2:48:")]
#[case("viewTransitionClass({old:{opacity:[0,1]}})", "2:55:")]
#[case("positionTry({ top:1}, {})", "2:14:")]
#[case("positionTry()", "2:14:")]
#[case("viewTransitionClass(runtime)", "2:14:")]
#[serial]
fn invalid_inputs_are_located_when_the_call_cannot_lower(
    #[case] call: &str,
    #[case] location: &str,
) {
    // Given
    reset();
    let source = format!("import stylex from '@stylexjs/stylex';\nconst result=stylex.{call};");
    // When
    let error = match crate::extract("/src/errors.tsx", &source, crate::ExtractOption::default()) {
        Err(error) => error.to_string(),
        Ok(output) => panic!("invalid transition compiled: {}", output.code),
    };
    // Then
    assert!(
        error.contains(&format!("/src/errors.tsx:{location}")),
        "{error}"
    );
    assert!(error.contains("cannot use"), "{error}");
}

#[rstest]
#[case("group", "true", "boolean true")]
#[case("imagePair", "() => 1", "a function")]
#[case("new", "function() { return 1; }", "a function")]
#[case("old", "runtime.opacity", "an unknown-at-build-time value")]
#[case("old", "getRuntime().opacity", "an unknown-at-build-time value")]
#[serial]
fn invalid_declarations_name_the_slot_and_kind_when_values_cannot_be_static(
    #[case] slot: &str,
    #[case] value: &str,
    #[case] kind: &str,
) {
    // Given
    reset();
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nconst result=stylex.viewTransitionClass({{{slot}:{{opacity:{value}}}}});"
    );
    // When
    let error = crate::extract("/src/errors.tsx", &source, crate::ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("invalid declaration compiled: {source}"))
        .to_string();
    // Then
    assert!(error.contains("/src/errors.tsx:2:"), "{error}");
    assert!(error.contains("stylex.viewTransitionClass"), "{error}");
    assert!(
        error.contains(&format!("slot `{slot}`, declaration `opacity` has {kind}")),
        "{error}"
    );
}

#[rstest]
#[case("keyframes()")]
#[case("keyframes({to:{opacity:0}}, {to:{opacity:1}})")]
#[serial]
fn keyframes_arity_is_rejected_when_the_dependency_cannot_lower(#[case] call: &str) {
    // Given
    reset();
    let source = format!("import stylex from '@stylexjs/stylex';\nconst fade=stylex.{call};");
    // When
    let error = crate::extract("/src/errors.tsx", &source, crate::ExtractOption::default())
        .err()
        .unwrap_or_else(|| panic!("invalid keyframes compiled: {source}"))
        .to_string();
    // Then
    assert!(error.contains("/src/errors.tsx:2:12:"), "{error}");
    assert!(error.contains("stylex.keyframes"), "{error}");
    assert!(error.contains("cannot use"), "{error}");
}
