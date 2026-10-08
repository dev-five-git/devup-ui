use rstest::rstest;
use serial_test::serial;

use super::{css, extract, names, reset};

#[rstest]
#[case("null")]
#[case("undefined")]
#[case("false")]
#[case("void 0")]
#[serial]
fn position_declarations_are_omitted_when_the_value_is_known_no_style(#[case] value: &str) {
    // Given
    reset();
    let source =
        format!("const a=stylex.positionTry({{top:{value}}});const b=stylex.positionTry({{}});");
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![format!("@position-try {}{{}}", values[0])]
    );
}

#[rstest]
#[case("null")]
#[case("undefined")]
#[case("false")]
#[serial]
fn view_declarations_are_omitted_when_the_value_is_known_no_style(#[case] value: &str) {
    // Given
    reset();
    let source = format!(
        "const a=stylex.viewTransitionClass({{old:{{opacity:{value}}}}});const b=stylex.viewTransitionClass({{old:{{}}}});"
    );
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![format!("::view-transition-old(*.{}){{}}", values[0])]
    );
}

#[rstest]
#[case("false")]
#[case("null")]
#[serial]
fn final_duplicate_omits_earlier_scalar_when_the_last_value_is_no_style(#[case] value: &str) {
    // Given
    reset();
    let source = format!(
        "const a=stylex.viewTransitionClass({{old:{{opacity:1,width:2,opacity:{value}}}}});const b=stylex.viewTransitionClass({{old:{{width:2}}}});"
    );
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![format!(
            "::view-transition-old(*.{}){{width:2px;}}",
            values[0]
        )]
    );
}

#[rstest]
#[case("false")]
#[case("null")]
#[serial]
fn reinsertion_keeps_first_position_when_an_omitted_duplicate_is_overwritten(#[case] value: &str) {
    // Given
    reset();
    let source = format!(
        "const a=stylex.viewTransitionClass({{old:{{opacity:1,width:2,opacity:{value},opacity:0}}}});const b=stylex.viewTransitionClass({{old:{{opacity:0,width:2}}}});"
    );
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![format!(
            "::view-transition-old(*.{}){{opacity:0;width:2px;}}",
            values[0]
        )]
    );
}

#[rstest]
#[case("true", "boolean true")]
#[case("[0,1]", "an array")]
#[case("{ default:0}", "an object")]
#[serial]
fn unsupported_static_values_are_located_when_the_slot_requires_scalars(
    #[case] value: &str,
    #[case] kind: &str,
) {
    // Given
    reset();
    let source = format!(
        "import stylex from '@stylexjs/stylex';\nconst a=stylex.viewTransitionClass({{old:{{opacity:{value}}}}});"
    );
    // When
    let error = match crate::extract("/src/invalid.tsx", &source, crate::ExtractOption::default()) {
        Err(error) => error.to_string(),
        Ok(output) => panic!("unsupported value compiled: {}", output.code),
    };
    // Then
    assert!(error.contains("/src/invalid.tsx:2:50:"), "{error}");
    assert!(error.contains("stylex.viewTransitionClass()"), "{error}");
    assert!(
        error.contains("slot `old`, declaration `opacity`"),
        "{error}"
    );
    assert!(error.contains(kind), "{error}");
    assert!(error.contains("static strings/numbers"), "{error}");
}

#[test]
#[serial]
fn shadowed_undefined_is_unknown_when_the_identifier_is_a_parameter() {
    // Given
    reset();
    let source = "import stylex from '@stylexjs/stylex';\nfunction render(undefined){return stylex.viewTransitionClass({old:{opacity:undefined}})}";
    // When
    let error = match crate::extract("/src/shadowed.tsx", source, crate::ExtractOption::default()) {
        Err(error) => error.to_string(),
        Ok(output) => panic!("shadowed undefined compiled: {}", output.code),
    };
    // Then
    assert!(error.contains("unknown-at-build-time"), "{error}");
    assert!(error.contains("declaration `opacity`"), "{error}");
}

#[test]
#[serial]
fn shorthand_fallback_arrays_are_located_when_position_requires_scalars() {
    // Given
    reset();
    let source = "import stylex from '@stylexjs/stylex';\nconst a=stylex.positionTry({margin:['1px','2px']});";
    // When
    let error = match crate::extract("/src/fallback.tsx", source, crate::ExtractOption::default()) {
        Err(error) => error.to_string(),
        Ok(output) => panic!("shorthand array compiled: {}", output.code),
    };
    // Then
    assert!(error.contains("/src/fallback.tsx:2:36:"), "{error}");
    assert!(error.contains("stylex.positionTry()"), "{error}");
    assert!(
        error.contains("slot `position`, declaration `margin` has an array"),
        "{error}"
    );
}

#[test]
#[serial]
fn static_false_constants_are_omitted_when_the_evaluator_resolves_them() {
    // Given
    reset();
    // When
    let output = extract(
        "const OFF=false;const a=stylex.viewTransitionClass({old:{opacity:OFF}});const b=stylex.viewTransitionClass({old:{}});",
    );
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![format!("::view-transition-old(*.{}){{}}", values[0])]
    );
}
