use rstest::rstest;
use serial_test::serial;

use super::{css, extract, names, reset};

fn seed_atoms() {
    let output = crate::extract(
        "/unrelated.tsx",
        "import {Box} from '@devup-ui/react';const node=<Box color='red' width='17px' height='29px'/>;",
        crate::ExtractOption {
            single_css: true,
            ..crate::ExtractOption::default()
        },
    );
    let output = output.unwrap_or_else(|error| panic!("atom seed: {error}"));
    assert_eq!(output.styles.len(), 3);
}

#[rstest]
#[case(
    "const fade=stylex.keyframes({to:{opacity:0}});const a=stylex.viewTransitionClass({old:{animationName:fade}});"
)]
#[case(
    "const a=stylex.viewTransitionClass({old:{animationName:stylex.keyframes({to:{opacity:0}})}});"
)]
#[case(
    "const vars=stylex.defineVars({duration:'1s'});const a=stylex.viewTransitionClass({old:{animationDuration:vars.duration}});"
)]
#[serial]
fn dependency_css_is_stable_when_unrelated_atoms_arrive_first(#[case] source: &str) {
    // Given
    reset();
    let expected = extract(source);
    reset();
    seed_atoms();
    // When
    let output = extract(source);
    // Then
    assert_eq!(names(&output), names(&expected));
    assert_eq!(css(&output), css(&expected));
}

#[test]
#[serial]
fn transition_identity_is_stable_when_keyframe_definitions_are_reordered() {
    // Given
    reset();
    let expected = extract(
        "const fade=stylex.keyframes({to:{opacity:0}});const grow=stylex.keyframes({to:{width:'16px'}});const a=stylex.viewTransitionClass({old:{animationName:fade}});",
    );
    reset();
    // When
    let output = extract(
        "const grow=stylex.keyframes({to:{width:'16px'}});const fade=stylex.keyframes({to:{opacity:0}});const a=stylex.viewTransitionClass({old:{animationName:fade}});",
    );
    // Then
    assert_eq!(names(&output)[2], names(&expected)[2]);
    assert_eq!(css(&output), css(&expected));
}

#[test]
#[serial]
fn emitted_dependency_matches_scalar_when_keyframes_are_referenced_and_consumed() {
    // Given
    reset();
    // When
    let output = extract(
        "const fade=stylex.keyframes({to:{opacity:0}});const a=stylex.viewTransitionClass({old:{animationName:fade}});const s=stylex.create({base:{animationName:fade,viewTransitionClass:a}});",
    );
    // Then
    let values = names(&output);
    assert!(css(&output).contains(&format!("@keyframes {}{{to{{opacity:0;}}}}", values[0])));
    assert!(css(&output).contains(&format!(
        "::view-transition-old(*.{}){{animation-name:{};}}",
        values[1], values[0]
    )));
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Static(style) if style.property == "animation-name" && style.value == values[0])));
}

#[test]
#[serial]
fn inline_keyframes_share_when_the_semantic_frame_content_matches() {
    // Given
    reset();
    // When
    let output = extract(
        "const fade=stylex.keyframes({to:{opacity:0}});const a=stylex.viewTransitionClass({old:{animationName:fade}});const b=stylex.viewTransitionClass({old:{animationName:stylex.keyframes({to:{opacity:0}})}});",
    );
    // Then
    let values = names(&output);
    assert_eq!(values[1], values[2]);
    assert_eq!(css(&output).len(), 2);
    assert!(css(&output).contains(&format!("@keyframes {}{{to{{opacity:0;}}}}", values[0])));
}

#[test]
#[serial]
fn vars_match_dependent_css_when_the_returned_reference_is_consumed() {
    // Given
    reset();
    // When
    let output = extract(
        "const vars=stylex.defineVars({duration:'1s'});const a=stylex.viewTransitionClass({old:{animationDuration:vars.duration}});const s=stylex.create({base:{animationDuration:vars.duration}});",
    );
    // Then
    let rules = css(&output);
    let variable = rules
        .iter()
        .find_map(|rule| {
            rule.strip_prefix(":root{")
                .and_then(|body| body.split_once(':').map(|(key, _)| key))
        })
        .unwrap_or_else(|| panic!("missing root variable: {rules:?}"));
    assert!(rules.contains(&format!(
        "::view-transition-old(*.{}){{animation-duration:var({variable});}}",
        names(&output)[0]
    )));
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Static(style) if style.property == "animation-duration" && style.value == format!("var({variable})"))));
}
