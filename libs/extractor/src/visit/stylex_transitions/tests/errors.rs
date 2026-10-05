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
