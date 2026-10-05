use super::{css, extract, names, reset};
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn pseudo_selectors_are_exact_when_all_four_slots_are_present() {
    // Given
    reset();
    // When
    let output = extract(
        "const a=stylex.viewTransitionClass({group:{animationDuration:300},imagePair:{isolation:'isolate'},old:{opacity:0},new:{fontWeight:700,borderRadius:16}});",
    );
    // Then
    let name = &names(&output)[0];
    assert_eq!(
        css(&output),
        vec![format!(
            "::view-transition-group(*.{name}){{animation-duration:300ms;}}::view-transition-image-pair(*.{name}){{isolation:isolate;}}::view-transition-old(*.{name}){{opacity:0;}}::view-transition-new(*.{name}){{font-weight:700;border-radius:16px;}}"
        )]
    );
}

#[test]
#[serial]
fn transition_names_are_scalars_when_styles_reference_returned_names() {
    // Given
    reset();
    // When
    let output = extract(
        "const fallback=stylex.positionTry({ top:1});const transition=stylex.viewTransitionClass({old:{opacity:0}});const s=stylex.create({base:{positionTryStyles:fallback,viewTransitionClass:transition}});",
    );
    // Then
    let values = names(&output);
    let declarations: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => {
                Some((style.property.clone(), style.value.clone()))
            }
            _ => None,
        })
        .collect();
    assert!(
        declarations.contains(&("position-try-styles".into(), values[0].clone())),
        "{declarations:?}"
    );
    assert!(declarations.contains(&("view-transition-class".into(), values[1].clone())));
}

#[test]
#[serial]
fn keyframes_and_vars_resolve_when_transition_slots_reference_them() {
    // Given
    reset();
    // When
    let output = extract(
        "const vars=stylex.defineVars({duration:'1s'});const fade=stylex.keyframes({to:{opacity:0}});const a=stylex.viewTransitionClass({old:{animationName:fade,animationDuration:vars.duration}});const b=stylex.viewTransitionClass({old:{animationName:stylex.keyframes({to:{opacity:0}}),animationDuration:vars.duration}});const s=stylex.create({base:{viewTransitionClass:b}});",
    );
    // Then
    let values = names(&output);
    assert_eq!(values[1], values[2]);
    assert!(
        css(&output)
            .iter()
            .any(|css| css.contains("animation-duration:var(--"))
    );
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Static(style) if style.property == "view-transition-class" && style.value == values[2])));
}

#[rstest]
#[case("positionTry", "{top:SIZE}", "{top:4}")]
#[case("viewTransitionClass", "{old:{opacity:SIZE}}", "{old:{opacity:4}}")]
#[serial]
fn static_constants_share_when_evaluation_resolves_them(
    #[case] api: &str,
    #[case] expression: &str,
    #[case] literal: &str,
) {
    // Given
    reset();
    let source = format!(
        "const SIZE=2+2;const a=stylex.{api}({expression});const b=stylex.{api}({literal});"
    );
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
}

#[test]
#[serial]
fn shadowed_functions_survive_when_the_binding_is_not_stylex() {
    // Given
    reset();
    // When
    let output = extract(
        "function render(stylex){return stylex.positionTry({color:'red'})}const a=stylex.positionTry({top:1});",
    );
    // Then
    assert!(output.code.contains("return stylex.positionTry"));
    assert_eq!(css(&output).len(), 1);
}

#[test]
#[serial]
fn anchor_names_remain_identifiers_when_position_rules_reference_them() {
    // Given
    reset();
    // When
    let output = extract(
        "const a=stylex.positionTry({positionAnchor:'--target',anchorName:'--self',width:16});",
    );
    // Then
    assert_eq!(
        css(&output),
        vec![format!(
            "@position-try {}{{anchor-name:--self;position-anchor:--target;width:16px;}}",
            names(&output)[0]
        )]
    );
}

#[test]
#[serial]
fn inline_position_names_are_scalars_when_create_contains_the_call() {
    // Given
    reset();
    // When
    let output =
        extract("const s=stylex.create({base:{positionTryStyles:stylex.positionTry({ top:1})}});");
    // Then
    assert!(output.styles.iter().any(|style| matches!(style, crate::ExtractStyleValue::Static(style) if style.property == "position-try-styles" && style.value.starts_with("--sxp-"))));
}

#[test]
#[serial]
fn unknown_css_properties_remain_scalars_when_view_slots_are_flat() {
    // Given
    reset();
    // When
    let output = extract("const a=stylex.viewTransitionClass({old:{futureProperty:'static'}});");
    // Then
    assert_eq!(
        css(&output),
        vec![format!(
            "::view-transition-old(*.{}){{future-property:static;}}",
            names(&output)[0]
        )]
    );
}
