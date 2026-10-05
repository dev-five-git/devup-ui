use super::{css, extract, names, reset};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("positionTry", "{top:1}", "{top:2}")]
#[case("viewTransitionClass", "{old:{opacity:0}}", "{old:{opacity:1}}")]
#[serial]
fn distinct_payloads_have_distinct_names_when_calls_share_a_file(
    #[case] api: &str,
    #[case] first: &str,
    #[case] second: &str,
) {
    // Given
    reset();
    let source = format!(
        "const a=stylex.{api}({first});const b=stylex.{api}({second});const c=stylex.{api}({first});"
    );
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_ne!(values[0], values[1]);
    assert_eq!(values[0], values[2]);
    assert_eq!(css(&output).len(), 2);
}

#[rstest]
#[case("positionTry", "{top:1,width:2}", "{width:2,top:1}", true)]
#[case(
    "viewTransitionClass",
    "{old:{opacity:0,width:2}}",
    "{old:{width:2,opacity:0}}",
    false
)]
#[case("viewTransitionClass", "{old:{},new:{}}", "{new:{},old:{}}", false)]
#[serial]
fn sharing_matches_serialization_when_entries_are_reordered(
    #[case] api: &str,
    #[case] first: &str,
    #[case] second: &str,
    #[case] shares: bool,
) {
    // Given
    reset();
    let source = format!("const a=stylex.{api}({first});const b=stylex.{api}({second});");
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0] == values[1], shares);
}

#[test]
#[serial]
fn empty_slots_are_distinct_when_the_view_body_is_empty() {
    // Given
    reset();
    // When
    let output = extract(
        "const a=stylex.viewTransitionClass({});const b=stylex.viewTransitionClass({old:{}});const c=stylex.positionTry({});",
    );
    // Then
    let values = names(&output);
    assert_ne!(values[0], values[1]);
    assert_eq!(
        css(&output),
        vec![
            format!("::view-transition-old(*.{}){{}}", values[1]),
            format!("@position-try {}{{}}", values[2])
        ]
    );
}

#[test]
#[serial]
fn names_are_stable_when_unrelated_extractions_arrive_first() {
    // Given
    reset();
    let expected = names(&extract(
        "const a=stylex.positionTry({ top:1});const b=stylex.viewTransitionClass({old:{opacity:0}});",
    ));
    let _unrelated = crate::extract("/other.tsx", "import stylex from '@stylexjs/stylex';const a=stylex.positionTry({ left:8});const b=stylex.keyframes({to:{opacity:1}});", crate::ExtractOption::default()).unwrap_or_else(|error| panic!("unrelated compilation: {error}"));
    // When
    let output = extract(
        "const b=stylex.viewTransitionClass({old:{opacity:0}});const a=stylex.positionTry({ top:1});",
    );
    // Then
    assert_eq!(
        names(&output),
        vec![expected[1].clone(), expected[0].clone()]
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn names_are_css_safe_when_prefix_and_debug_are_configured(#[case] debug: bool) {
    // Given
    reset();
    css::set_prefix(Some("du-".into()));
    css::debug::set_debug(debug);
    // When
    let output = extract(
        "const a=stylex.positionTry({ top:1});const b=stylex.viewTransitionClass({old:{opacity:0}});",
    );
    // Then
    let values = names(&output);
    assert!(values[0].starts_with("--du-sxp-"));
    assert!(values[1].starts_with("du-sxv-"));
    assert!(values.iter().all(|name| {
        name.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }));
    reset();
}
